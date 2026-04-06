use anyhow::{Context, Result, bail};
use gltf::Document;
use ndarray::Array2;
use ndarray_npy::{read_npy, write_npy};
use std::path::{Path, PathBuf};
use tch::nn::OptimizerConfig;
use tch::{Device, IValue, IndexOp, Kind, Tensor, nn};

#[derive(clap::Args, Debug)]
/// CLI arguments for mesh fitting and training.
pub struct TrainArgs {
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
/// Metadata stored next to each rendered view.
struct Meta {
    out_size: i64,
}

/// One SDF observation with image dimensions.
struct ViewObs {
    w: f64,
    h: f64,
    sdf: Tensor, // [1,1,H,W] float32, <0 inside, >0 outside
}

/// Loaded set of SDF observations used for optimization.
struct ViewSet {
    items: Vec<ViewObs>,
}

impl ViewSet {
    /// Loads all valid view subfolders and optionally truncates the count.
    fn load(dir: &Path, num_views: usize, device: Device) -> Result<Self> {
        // let mut subdirs: Vec<_> = std::fs::read_dir(dir)?
        //     .filter_map(|e| e.ok())
        //     .map(|e| e.path())
        //     .filter(|p| p.is_dir())
        //     .collect();
        // subdirs.sort();
        //
        // let mut items = Vec::new();
        // for sd in subdirs {
        //     let meta_path = sd.join("meta.json");
        //     let sdf_path = sd.join("sdf.npy");
        //     if !(meta_path.exists() && sdf_path.exists()) {
        //         continue;
        //     }
        //
        //     let meta: Meta = serde_json::from_reader(std::fs::File::open(&meta_path)?)?;
        //     let size = meta.out_size;
        //
        //     let sdf: Array2<f32> =
        //         read_npy(&sdf_path).with_context(|| format!("read {:?}", sdf_path))?;
        //     let (h, w) = (sdf.shape()[0] as i64, sdf.shape()[1] as i64);
        //     let (sdf_vec, offset) = sdf.into_raw_vec_and_offset();
        //     if offset.is_some_and(|x| x != 0) {
        //         bail!("unexpected non-zero ndarray offset in {:?}", sdf_path);
        //     }
        //     let sdf_t = Tensor::from_slice(&sdf_vec)
        //         .view([1, 1, h, w])
        //         .to_device(device);
        //
        //     items.push(ViewObs {
        //         w: size as f64,
        //         h: size as f64,
        //         sdf: sdf_t,
        //     });
        // }
        //
        // if items.is_empty() {
        //     bail!("no views found under {:?}", dir);
        // }
        // if num_views > 0 && items.len() > num_views {
        //     items.truncate(num_views);
        // }
        // Ok(Self { items })
        todo!()
    }

    /// Returns the number of loaded observations.
    fn len(&self) -> usize {
        self.items.len()
    }
}

/// GLB-derived tensors and indices needed for linear-blend skinning.
struct SkinnedMeshData {
    v_count: i64,
    j_count: i64,
    bind_global: Tensor, // [J,4,4]
    ibm: Tensor,         // [J,4,4]
    v0h: Tensor,         // [V,4]
    j_idx: Tensor,       // [V,4]
    w: Tensor,           // [V,4] normalized
    fit_joints: Vec<i64>,
    device: Device,
}

impl SkinnedMeshData {
    /// Parses a skinned GLB and builds tensors used throughout optimization.
    fn load(glb_path: &Path, max_joints_to_fit: usize, device: Device) -> Result<Self> {
        let (doc, buffers, _) =
            gltf::import(glb_path).with_context(|| format!("import {:?}", glb_path))?;

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

        let skin = doc.skins().next().context("no skin in glb")?;
        let joint_nodes: Vec<_> = skin.joints().collect();
        let j_count = joint_nodes.len() as i64;

        let skin_reader = skin.reader(|b| Some(&buffers[b.index()]));
        let ibm_mats: Vec<[[f32; 4]; 4]> = skin_reader
            .read_inverse_bind_matrices()
            .context("failed to read ibm")?
            .collect();
        if ibm_mats.len() as i64 != j_count {
            bail!("inverse bind matrices count != joints count");
        }

        let parents = build_parents(&doc);
        let mut bind_global = Vec::new();
        for jn in &joint_nodes {
            bind_global.push(global_matrix(&doc, &parents, jn.index(), device));
        }
        let bind_global = Tensor::stack(&bind_global, 0);

        let ibm = {
            let mut ms = Vec::new();
            for m in ibm_mats {
                let flat: Vec<f32> = m.iter().flat_map(|r| r.iter().copied()).collect();
                ms.push(Tensor::from_slice(&flat).view([4, 4]).to_device(device));
            }
            Tensor::stack(&ms, 0)
        };

        let v0: Vec<f32> = positions.iter().flat_map(|p| [p[0], p[1], p[2]]).collect();
        let v0 = Tensor::from_slice(&v0).view([v_count, 3]).to_device(device);
        let ones = Tensor::ones([v_count, 1], (Kind::Float, device));
        let v0h = Tensor::cat(&[v0, ones], 1);

        let j_flat: Vec<i64> = joints
            .iter()
            .flat_map(|j| j.iter().map(|&x| x as i64))
            .collect();
        let w_flat: Vec<f32> = weights.iter().flat_map(|w| w.iter().copied()).collect();
        let j_idx = Tensor::from_slice(&j_flat)
            .view([v_count, 4])
            .to_device(device);
        let w = Tensor::from_slice(&w_flat)
            .view([v_count, 4])
            .to_device(device);
        let w = &w / (w.sum_dim_intlist([1].as_slice(), true, Kind::Float) + 1e-8);

        let fit_joints = pick_major_joints(&joints, &weights, j_count as usize, max_joints_to_fit);

        Ok(Self {
            v_count,
            j_count,
            bind_global,
            ibm,
            v0h,
            j_idx,
            w,
            fit_joints,
            device,
        })
    }

    /// Returns how many joints are actively optimized.
    fn fit_joint_count(&self) -> usize {
        self.fit_joints.len()
    }

    /// Applies current joint scales and returns skinned vertices as `[V,3]`.
    fn skin_vertices(&self, log_sxz: &Tensor) -> Tensor {
        let joint_scale = self.build_joint_scale_matrices(log_sxz);
        let m = self.bind_global.matmul(&joint_scale).matmul(&self.ibm);

        let j_flat = self.j_idx.reshape([-1]);
        let m_sel = m.index_select(0, &j_flat).view([self.v_count, 4, 4, 4]);
        let v = self.v0h.unsqueeze(1).unsqueeze(-1);
        let tv = m_sel.matmul(&v).squeeze_dim(-1);
        let tv_xyz = tv.i((.., .., 0..3));

        (tv_xyz * self.w.unsqueeze(-1)).sum_dim_intlist([1].as_slice(), false, Kind::Float)
    }

    /// Builds per-joint homogeneous scale matrices `[J,4,4]`.
    fn build_joint_scale_matrices(&self, log_sxz: &Tensor) -> Tensor {
        let sxz = log_sxz.exp();
        let sx = sxz.i((.., 0));
        let sz = sxz.i((.., 1));

        let diag = Tensor::ones([self.j_count, 3], (Kind::Float, self.device));
        for (slot, &joint) in self.fit_joints.iter().enumerate() {
            diag.i((joint, 0)).copy_(&sx.i(slot as i64));
            diag.i((joint, 2)).copy_(&sz.i(slot as i64));
        }

        let zeros = Tensor::zeros([self.j_count, 1], (Kind::Float, self.device));
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
                Tensor::ones([self.j_count, 1], (Kind::Float, self.device)),
            ],
            1,
        );
        Tensor::stack(&[row0, row1, row2, row3], 1)
    }
}

/// Wrapper around the TorchScript module that samples SDF grids.
struct SdfSampler {
    module: tch::CModule,
}

impl SdfSampler {
    /// Loads the TorchScript sampler onto the selected device.
    fn load(path: &Path, device: Device) -> Result<Self> {
        let module = tch::CModule::load_on_device(path, device)
            .with_context(|| format!("load sampler {:?}", path))?;
        Ok(Self { module })
    }

    /// Evaluates sampled SDF values for a precomputed sampling grid.
    fn sample(&self, sdf: &Tensor, grid: Tensor) -> Result<Tensor> {
        let out = self
            .module
            .forward_is(&[IValue::Tensor(sdf.shallow_clone()), IValue::Tensor(grid)])?;
        match out {
            IValue::Tensor(t) => Ok(t),
            _ => bail!("sampler output not tensor"),
        }
    }
}

/// Trainable per-view camera parameters used for silhouette alignment.
struct CameraParams {
    yaw: Tensor,
    log_s: Tensor,
    tx: Tensor,
    ty: Tensor,
}

impl CameraParams {
    /// Allocates camera parameters and spreads initial yaw angles around 360 degrees.
    fn new(root: &nn::Path<'_>, num_views: i64, device: Device) -> Self {
        let yaw = root.var("yaw", &[num_views], nn::Init::Const(0.0));
        let log_s = root.var("log_s", &[num_views], nn::Init::Const((800.0f64).ln()));
        let tx = root.var("tx", &[num_views], nn::Init::Const(512.0));
        let ty = root.var("ty", &[num_views], nn::Init::Const(512.0));

        tch::no_grad(|| {
            for i in 0..num_views {
                let ang = (2.0 * std::f64::consts::PI) * (i as f64) / (num_views as f64);
                yaw.i(i).copy_(&Tensor::from(ang as f32).to_device(device));
            }
        });

        Self { yaw, log_s, tx, ty }
    }
}

/// Full mutable training state containing optimization variables.
struct TrainState {
    vs: nn::VarStore,
    log_sxz: Tensor,
    cameras: CameraParams,
}

impl TrainState {
    /// Initializes optimization variables for joint scales and camera parameters.
    fn new(device: Device, fit_joint_count: usize, num_views: i64) -> Self {
        let vs = nn::VarStore::new(device);
        let root = vs.root();
        let log_sxz = root.var(
            "log_sxz",
            &[fit_joint_count as i64, 2],
            nn::Init::Const(0.0),
        );
        let cameras = CameraParams::new(&root, num_views, device);
        Self {
            vs,
            log_sxz,
            cameras,
        }
    }
}

/// Top-level application object orchestrating load, train, and export.
struct MeshTrainApp {
    args: TrainArgs,
    device: Device,
    mesh: SkinnedMeshData,
    views: ViewSet,
    sampler: SdfSampler,
    train: TrainState,
}

impl MeshTrainApp {
    /// Builds the application from CLI args and loads all required assets.
    fn from_args(args: TrainArgs) -> Result<Self> {
        let device = Device::cuda_if_available();
        let mesh = SkinnedMeshData::load(&args.glb, args.max_joints_to_fit, device)?;
        let views = ViewSet::load(&args.views_dir, args.num_views, device)?;
        let sampler = SdfSampler::load(&args.sdf_sampler, device)?;
        let train = TrainState::new(device, mesh.fit_joint_count(), views.len() as i64);
        Ok(Self {
            args,
            device,
            mesh,
            views,
            sampler,
            train,
        })
    }

    /// Executes optimization and writes the fitted vertex output.
    fn run(mut self) -> Result<()> {
        self.optimize()?;
        self.export_vertices()?;
        Ok(())
    }

    /// Runs the main gradient-descent loop over all iterations.
    fn optimize(&mut self) -> Result<()> {
        let mut opt = nn::Adam::default().build(&self.train.vs, self.args.lr)?;

        for it in 0..self.args.iters {
            let v_skin = self.mesh.skin_vertices(&self.train.log_sxz);
            let sil = self.silhouette_loss(&v_skin)?;
            let reg = self.train.log_sxz.pow_tensor_scalar(2.0).mean(Kind::Float);
            let loss = self.args.w_sil * &sil + self.args.w_reg * &reg;

            opt.backward_step(&loss);

            if it % 50 == 0 || it + 1 == self.args.iters {
                let l = loss.to_device(Device::Cpu).double_value(&[]);
                let s = sil.to_device(Device::Cpu).double_value(&[]);
                let r = reg.to_device(Device::Cpu).double_value(&[]);
                eprintln!("it {it:4} loss={l:.6} sil={s:.6} reg={r:.6}");
            }
        }
        Ok(())
    }

    /// Computes average silhouette penalty across all views.
    fn silhouette_loss(&self, v_skin: &Tensor) -> Result<Tensor> {
        let mut sil = Tensor::zeros([], (Kind::Float, self.device));

        for (i, view) in self.views.items.iter().enumerate() {
            let idx = i as i64;
            let r = roty(&self.train.cameras.yaw.i(idx));
            let v_rot = v_skin.matmul(&r.transpose(0, 1));

            let s = self.train.cameras.log_s.i(idx).exp();
            let u = &s * v_rot.i((.., 0)) + self.train.cameras.tx.i(idx);
            let vv = &s * (-v_rot.i((.., 1))) + self.train.cameras.ty.i(idx);

            let w_img = view.w - 1.0;
            let h_img = view.h - 1.0;
            let x = (&u / w_img) * 2.0 - 1.0;
            let y = (&vv / h_img) * 2.0 - 1.0;
            let grid = Tensor::stack(&[x, y], 1).view([1, self.mesh.v_count, 1, 2]);

            let sdf_vals = self
                .sampler
                .sample(&view.sdf, grid)?
                .view([self.mesh.v_count]);
            sil += sdf_vals.relu().mean(Kind::Float);
        }
        Ok(sil / self.views.len() as f64)
    }

    /// Exports final skinned vertices as an `Nx3` `.npy` file.
    fn export_vertices(&self) -> Result<()> {
        tch::no_grad(|| -> Result<()> {
            // let v_skin = self.mesh.skin_vertices(&self.train.log_sxz);
            // let v_cpu = v_skin.to_device(Device::Cpu);
            // let flat: Vec<f32> = Vec::<f32>::try_from(&v_cpu.reshape([-1]))?;
            //
            // let n = flat.len() / 3;
            // let mut arr = Array2::<f32>::zeros((n, 3));
            // for i in 0..n {
            //     arr[(i, 0)] = flat[3 * i];
            //     arr[(i, 1)] = flat[3 * i + 1];
            //     arr[(i, 2)] = flat[3 * i + 2];
            // }
            //
            // if let Some(parent) = self.args.out_npy.parent() {
            //     std::fs::create_dir_all(parent)?;
            // }
            // write_npy(&self.args.out_npy, &arr)?;
            // eprintln!("wrote {:?}", self.args.out_npy);
            // Ok(())
            todo!()
        })
    }
}

/// Selects the joints with highest accumulated skinning weight mass.
fn pick_major_joints(
    joints: &[[u16; 4]],
    weights: &[[f32; 4]],
    joint_count: usize,
    max_joints_to_fit: usize,
) -> Vec<i64> {
    let mut mass = vec![0f64; joint_count];
    for (ji4, wi4) in joints.iter().zip(weights.iter()) {
        for k in 0..4 {
            mass[ji4[k] as usize] += wi4[k] as f64;
        }
    }

    let mut order: Vec<usize> = (0..mass.len()).collect();
    order.sort_by(|&a, &b| mass[b].partial_cmp(&mass[a]).unwrap());

    let k = max_joints_to_fit.min(order.len());
    let top5: Vec<usize> = order.iter().take(5).copied().collect();
    eprintln!(
        "Fitting {k} joints (by weight mass). Top-5 masses: {:?}",
        top5
    );
    order[..k].iter().map(|&x| x as i64).collect()
}

/// Converts a node's local transform matrix to a device tensor.
fn node_local_matrix(n: &gltf::Node, device: Device) -> Tensor {
    let m = n.transform().matrix(); // [[f32;4];4] row-major
    let flat: Vec<f32> = m.iter().flat_map(|r| r.iter().copied()).collect();
    Tensor::from_slice(&flat).view([4, 4]).to_device(device)
}

/// Builds a `node -> parent` lookup table for the document graph.
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

/// Composes local transforms from root to a node to obtain a global matrix.
fn global_matrix(
    doc: &Document,
    parents: &[Option<usize>],
    node_idx: usize,
    device: Device,
) -> Tensor {
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

/// Creates a Y-axis rotation matrix from a yaw angle.
fn roty(yaw: &Tensor) -> Tensor {
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

/// Runs mesh training workflow for the provided CLI arguments.
pub fn run(args: TrainArgs) -> Result<()> {
    MeshTrainApp::from_args(args)?.run()
}
