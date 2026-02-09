use anyhow::{bail, Context, Result};
use clap::Parser;
use gltf::Document;
use ndarray::Array2;
use ndarray_npy::{read_npy, write_npy};
use std::path::{Path, PathBuf};
use tch::nn::OptimizerConfig;
use tch::{nn, Device, IValue, IndexOp, Kind, Tensor};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    glb: PathBuf,
    #[arg(long)]
    views_dir: PathBuf,
    #[arg(long)]
    sdf_sampler: PathBuf,
    #[arg(long)]
    out_npy: PathBuf,

    #[arg(long, default_value_t = 6)]
    num_views: usize,
    #[arg(long, default_value_t = 32)]
    max_joints_to_fit: usize,
    #[arg(long, default_value_t = 2000)]
    iters: i64,
    #[arg(long, default_value_t = 3e-2)]
    lr: f64,

    #[arg(long, default_value_t = 6.0)]
    w_sil: f64,
    #[arg(long, default_value_t = 5e-3)]
    w_reg: f64,
}

#[derive(serde::Deserialize)]
struct Meta {
    out_size: i64,
}

struct ViewObs {
    w: f64,
    h: f64,
    sdf: Tensor, // [1,1,H,W] float32, <0 inside, >0 outside
}

fn load_views(dir: &Path, device: Device) -> Result<Vec<ViewObs>> {
    let mut subdirs: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    subdirs.sort();

    let mut views = Vec::new();
    for sd in subdirs {
        let meta_path = sd.join("meta.json");
        let sdf_path = sd.join("sdf.npy");
        if !(meta_path.exists() && sdf_path.exists()) {
            continue;
        }

        let meta: Meta = serde_json::from_reader(std::fs::File::open(&meta_path)?)?;
        let size = meta.out_size;

        let sdf: Array2<f32> =
            read_npy(&sdf_path).with_context(|| format!("read {:?}", sdf_path))?;
        let (h, w) = (sdf.shape()[0] as i64, sdf.shape()[1] as i64);
        let (sdf_vec, offset) = sdf.into_raw_vec_and_offset();
        if offset.is_some_and(|x| x != 0) {
            bail!("unexpected non-zero ndarray offset in {:?}", sdf_path);
        }
        let sdf_t = Tensor::from_slice(&sdf_vec)
            .view([1, 1, h, w])
            .to_device(device);

        views.push(ViewObs {
            w: size as f64,
            h: size as f64,
            sdf: sdf_t,
        });
    }
    if views.is_empty() {
        bail!("no views found under {:?}", dir);
    }
    Ok(views)
}

fn mat4_from_cols(cols: [[f32; 4]; 4], device: Device) -> Tensor {
    // glTF uses column-major matrices conceptually; we store as [4,4] row-major Tensor
    // by transposing after building from columns.
    let flat: Vec<f32> = cols.iter().flat_map(|c| c.iter().copied()).collect();
    Tensor::from_slice(&flat)
        .view([4, 4])
        .transpose(0, 1)
        .to_device(device)
}

fn node_local_matrix(n: &gltf::Node, device: Device) -> Tensor {
    let m = n.transform().matrix(); // [[f32;4];4] row-major
    let flat: Vec<f32> = m.iter().flat_map(|r| r.iter().copied()).collect();
    Tensor::from_slice(&flat).view([4, 4]).to_device(device)
}

fn build_parents(doc: &Document) -> Vec<Option<usize>> {
    let mut parents = vec![None; doc.nodes().len()];
    for node in doc.nodes() {
        let p = node.index();
        for ch in node.children() {
            parents[ch.index()] = Some(p);
        }
    }
    parents
}

fn global_matrix(
    doc: &Document,
    parents: &[Option<usize>],
    node_idx: usize,
    device: Device,
) -> Tensor {
    // multiply parents up to root
    let mut chain = Vec::new();
    let mut cur = Some(node_idx);
    while let Some(i) = cur {
        chain.push(i);
        cur = parents[i];
    }
    chain.reverse();

    let mut m = Tensor::eye(4, (Kind::Float, device));
    for i in chain {
        let n = doc.nodes().nth(i).unwrap();
        m = m.matmul(&node_local_matrix(&n, device));
    }
    m
}

fn roty(yaw: &Tensor) -> Tensor {
    // yaw scalar -> [3,3]
    let c = yaw.cos();
    let s = yaw.sin();
    Tensor::stack(
        &[
            Tensor::stack(
                &[c.shallow_clone(), Tensor::zeros_like(&c), s.shallow_clone()],
                0,
            ),
            Tensor::stack(
                &[
                    Tensor::zeros_like(&c),
                    Tensor::ones_like(&c),
                    Tensor::zeros_like(&c),
                ],
                0,
            ),
            Tensor::stack(&[-s, Tensor::zeros_like(&c), c], 0),
        ],
        0,
    )
}

fn main() -> Result<()> {
    let a = Args::parse();
    let device = Device::cuda_if_available();

    // --- load glb ---
    let (doc, buffers, _) = gltf::import(&a.glb).with_context(|| format!("import {:?}", a.glb))?;

    // find first primitive that has skinning attributes
    let mut found = None;
    for mesh in doc.meshes() {
        for prim in mesh.primitives() {
            let reader = prim.reader(|b| Some(&buffers[b.index()]));
            if reader.read_positions().is_some()
                && reader.read_joints(0).is_some()
                && reader.read_weights(0).is_some()
            {
                found = Some(prim);
                break;
            }
        }
        if found.is_some() {
            break;
        }
    }
    let prim = found.context("no skinned primitive found in glb")?;

    let reader = prim.reader(|b| Some(&buffers[b.index()]));
    let positions: Vec<[f32; 3]> = reader.read_positions().unwrap().collect();
    let joints: Vec<[u16; 4]> = reader.read_joints(0).unwrap().into_u16().collect();
    let weights: Vec<[f32; 4]> = reader.read_weights(0).unwrap().into_f32().collect();
    let _indices: Vec<u32> = reader
        .read_indices()
        .context("mesh has no indices")?
        .into_u32()
        .collect();

    let v_count = positions.len() as i64;
    if joints.len() != positions.len() || weights.len() != positions.len() {
        bail!("positions/joints/weights length mismatch");
    }

    // locate the skin used by this mesh node:
    // In Blender exports, the skinned mesh is usually under a node that references a skin.
    // We'll pick the first skin in the file (common single-character case).
    let skin = doc.skins().next().context("no skin in glb")?;
    let joint_nodes: Vec<_> = skin.joints().collect();
    let j_count = joint_nodes.len() as i64;

    // inverse bind matrices
    let skin_reader = skin.reader(|b| Some(&buffers[b.index()]));
    let ibm_mats: Vec<[[f32; 4]; 4]> = skin_reader
        .read_inverse_bind_matrices()
        .context("failed to read ibm")?
        .collect();
    if ibm_mats.len() as i64 != j_count {
        bail!("inverse bind matrices count != joints count");
    }

    // global bind matrices for joints
    let parents = build_parents(&doc);
    let mut bind_global = Vec::new();
    for jn in &joint_nodes {
        bind_global.push(global_matrix(&doc, &parents, jn.index(), device));
    }
    let bind_global = Tensor::stack(&bind_global, 0); // [J,4,4]

    let ibm = {
        let mut ms = Vec::new();
        for m in ibm_mats {
            let flat: Vec<f32> = m.iter().flat_map(|r| r.iter().copied()).collect();
            ms.push(Tensor::from_slice(&flat).view([4, 4]).to_device(device));
        }
        Tensor::stack(&ms, 0) // [J,4,4]
    };

    // Base vertex tensor [V,3] (homogeneous later)
    let v0: Vec<f32> = positions.iter().flat_map(|p| [p[0], p[1], p[2]]).collect();
    let v0 = Tensor::from_slice(&v0).view([v_count, 3]).to_device(device);
    let ones = Tensor::ones([v_count, 1], (Kind::Float, device));
    let v0h = Tensor::cat(&[v0, ones], 1); // [V,4]

    // joints/weights tensors
    let j_flat: Vec<i64> = joints
        .iter()
        .flat_map(|j| j.iter().map(|&x| x as i64))
        .collect();
    let w_flat: Vec<f32> = weights.iter().flat_map(|w| w.iter().copied()).collect();
    let j_idx = Tensor::from_slice(&j_flat).view([v_count, 4]).to_device(device);
    let w = Tensor::from_slice(&w_flat).view([v_count, 4]).to_device(device);
    let w = &w / (w.sum_dim_intlist([1].as_slice(), true, Kind::Float) + 1e-8);

    // --- pick "major" joints automatically by weight mass ---
    let mut mass = vec![0f64; j_count as usize];
    for (ji4, wi4) in joints.iter().zip(weights.iter()) {
        for k in 0..4 {
            mass[ji4[k] as usize] += wi4[k] as f64;
        }
    }
    let mut order: Vec<usize> = (0..mass.len()).collect();
    order.sort_by(|&a, &b| mass[b].partial_cmp(&mass[a]).unwrap());
    let k = a.max_joints_to_fit.min(order.len());
    let fit_joints: Vec<i64> = order[..k].iter().map(|&x| x as i64).collect();
    eprintln!("Fitting {k} joints (by weight mass). Top-5 masses: {:?}", &order[..5]);

    // map joint -> slot (or -1)
    let mut slot = vec![-1i64; j_count as usize];
    for (s, &j) in fit_joints.iter().enumerate() {
        slot[j as usize] = s as i64;
    }

    // --- views ---
    let mut views = load_views(&a.views_dir, device)?;
    if a.num_views > 0 && views.len() > a.num_views {
        views.truncate(a.num_views);
    }
    let nv = views.len() as i64;

    // sampler
    let sdf_sampler = tch::CModule::load_on_device(&a.sdf_sampler, device)
        .with_context(|| format!("load sampler {:?}", a.sdf_sampler))?;

    // --- optimize parameters ---
    let vs = nn::VarStore::new(device);
    let root = vs.root();

    // per selected joint: log_sx, log_sz (Y fixed at 1)
    let log_sxz = root.var("log_sxz", &[k as i64, 2], nn::Init::Const(0.0));

    // per view camera: yaw, log_scale, tx, ty
    let yaw = root.var("yaw", &[nv], nn::Init::Const(0.0));
    let log_s = root.var("log_s", &[nv], nn::Init::Const((800.0f64).ln()));
    let tx = root.var("tx", &[nv], nn::Init::Const(512.0));
    let ty = root.var("ty", &[nv], nn::Init::Const(512.0));

    // init yaws to spread around
    tch::no_grad(|| {
        for i in 0..nv {
            let ang = (2.0 * std::f64::consts::PI) * (i as f64) / (nv as f64);
            yaw.i(i).copy_(&Tensor::from(ang as f32).to_device(device));
        }
    });

    let mut opt = nn::Adam::default().build(&vs, a.lr)?;

    for it in 0..a.iters {
        // build per-joint scale matrices S_j: default I
        // We’ll construct sxz for all joints and scatter from the K optimized ones.
        let sxz = log_sxz.exp(); // [K,2]
        let sx = sxz.i((.., 0));
        let sz = sxz.i((.., 1));

        let diag = Tensor::ones([j_count, 3], (Kind::Float, device));
        // scatter into diag for selected joints
        for (s, &j) in fit_joints.iter().enumerate() {
            diag.i((j, 0)).copy_(&sx.i(s as i64));
            diag.i((j, 2)).copy_(&sz.i(s as i64));
        }

        // S: [J,4,4]
        let zeros = Tensor::zeros([j_count, 1], (Kind::Float, device));
        let row0 = Tensor::cat(
            &[
                diag.i((.., 0)).unsqueeze(1),
                zeros.shallow_clone(),
                zeros.shallow_clone(),
                zeros.shallow_clone(),
            ],
            1,
        );
        let row1 = Tensor::cat(
            &[
                zeros.shallow_clone(),
                diag.i((.., 1)).unsqueeze(1),
                zeros.shallow_clone(),
                zeros.shallow_clone(),
            ],
            1,
        );
        let row2 = Tensor::cat(
            &[
                zeros.shallow_clone(),
                zeros.shallow_clone(),
                diag.i((.., 2)).unsqueeze(1),
                zeros.shallow_clone(),
            ],
            1,
        );
        let row3 = Tensor::cat(
            &[
                zeros.shallow_clone(),
                zeros.shallow_clone(),
                zeros.shallow_clone(),
                Tensor::ones([j_count, 1], (Kind::Float, device)),
            ],
            1,
        );
        let s_mat = Tensor::stack(&[row0, row1, row2, row3], 1); // [J,4,4] (rows)

        // joint transforms: M = bind_global * S * ibm
        let m = bind_global.matmul(&s_mat).matmul(&ibm); // [J,4,4]

        // skinning: gather per-vertex 4 joint matrices
        let j_flat = j_idx.reshape([-1]);
        let m_sel = m.index_select(0, &j_flat).view([v_count, 4, 4, 4]); // [V,4,4,4]
        let v = v0h.unsqueeze(1).unsqueeze(-1); // [V,1,4,1]
        let tv = m_sel.matmul(&v).squeeze_dim(-1); // [V,4,4]
        let tv_xyz = tv.i((.., .., 0..3)); // [V,4,3]
        let v_skin =
            (tv_xyz * w.unsqueeze(-1)).sum_dim_intlist([1].as_slice(), false, Kind::Float); // [V,3]

        // silhouette loss over views
        let mut sil = Tensor::zeros([], (Kind::Float, device));

        for (i, view) in views.iter().enumerate() {
            let i = i as i64;
            let r = roty(&yaw.i(i)); // [3,3]
            let v_rot = v_skin.matmul(&r.transpose(0, 1));

            let s = log_s.i(i).exp();
            let u = &s * v_rot.i((.., 0)) + tx.i(i);
            let vv = &s * (-v_rot.i((.., 1))) + ty.i(i);

            // normalize to [-1,1] for grid_sample
            let w_img = view.w - 1.0;
            let h_img = view.h - 1.0;
            let x = (&u / w_img) * 2.0 - 1.0;
            let y = (&vv / h_img) * 2.0 - 1.0;
            let grid = Tensor::stack(&[x, y], 1).view([1, v_count, 1, 2]);

            let out = sdf_sampler.forward_is(&[
                IValue::Tensor(view.sdf.shallow_clone()),
                IValue::Tensor(grid),
            ])?;
            let samp = match out {
                IValue::Tensor(t) => t,
                _ => bail!("sampler output not tensor"),
            };
            let sdf_vals = samp.view([v_count]);

            sil = sil + sdf_vals.relu().mean(Kind::Float); // penalize outside
        }
        sil /= views.len() as f64;

        let reg = log_sxz.pow_tensor_scalar(2.0).mean(Kind::Float);
        let loss = a.w_sil * &sil + a.w_reg * &reg;

        opt.backward_step(&loss);

        if it % 50 == 0 || it + 1 == a.iters {
            let l = loss.to_device(Device::Cpu).double_value(&[]);
            let s = sil.to_device(Device::Cpu).double_value(&[]);
            let r = reg.to_device(Device::Cpu).double_value(&[]);
            eprintln!("it {it:4} loss={l:.6} sil={s:.6} reg={r:.6}");
        }
    }

    // export fitted vertices (skinned rest with learned scales) as Nx3 .npy
    tch::no_grad(|| -> Result<()> {
        // rebuild final v_skin one more time (same as above but without grads)
        let sxz = log_sxz.exp();
        let sx = sxz.i((.., 0));
        let sz = sxz.i((.., 1));

        let diag = Tensor::ones([j_count, 3], (Kind::Float, device));
        for (s, &j) in fit_joints.iter().enumerate() {
            diag.i((j, 0)).copy_(&sx.i(s as i64));
            diag.i((j, 2)).copy_(&sz.i(s as i64));
        }

        let zeros = Tensor::zeros([j_count, 1], (Kind::Float, device));
        let row0 = Tensor::cat(
            &[
                diag.i((.., 0)).unsqueeze(1),
                zeros.shallow_clone(),
                zeros.shallow_clone(),
                zeros.shallow_clone(),
            ],
            1,
        );
        let row1 = Tensor::cat(
            &[
                zeros.shallow_clone(),
                diag.i((.., 1)).unsqueeze(1),
                zeros.shallow_clone(),
                zeros.shallow_clone(),
            ],
            1,
        );
        let row2 = Tensor::cat(
            &[
                zeros.shallow_clone(),
                zeros.shallow_clone(),
                diag.i((.., 2)).unsqueeze(1),
                zeros.shallow_clone(),
            ],
            1,
        );
        let row3 = Tensor::cat(
            &[
                zeros.shallow_clone(),
                zeros.shallow_clone(),
                zeros.shallow_clone(),
                Tensor::ones([j_count, 1], (Kind::Float, device)),
            ],
            1,
        );
        let s_mat = Tensor::stack(&[row0, row1, row2, row3], 1);

        let m = bind_global.matmul(&s_mat).matmul(&ibm);

        let j_flat = j_idx.reshape([-1]);
        let m_sel = m.index_select(0, &j_flat).view([v_count, 4, 4, 4]);
        let v = v0h.unsqueeze(1).unsqueeze(-1);
        let tv = m_sel.matmul(&v).squeeze_dim(-1);
        let tv_xyz = tv.i((.., .., 0..3));
        let v_skin =
            (tv_xyz * w.unsqueeze(-1)).sum_dim_intlist([1].as_slice(), false, Kind::Float);

        let v_cpu = v_skin.to_device(Device::Cpu);
        let flat: Vec<f32> = Vec::<f32>::try_from(&v_cpu.reshape([-1]))?;
        let n = flat.len() / 3;
        let mut arr = Array2::<f32>::zeros((n, 3));
        for i in 0..n {
            arr[(i, 0)] = flat[3 * i];
            arr[(i, 1)] = flat[3 * i + 1];
            arr[(i, 2)] = flat[3 * i + 2];
        }
        if let Some(parent) = a.out_npy.parent() {
            std::fs::create_dir_all(parent)?;
        }
        write_npy(&a.out_npy, &arr)?;
        eprintln!("wrote {:?}", a.out_npy);
        Ok(())
    })?;

    let _ = slot;
    Ok(())
}
