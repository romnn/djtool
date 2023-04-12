#![allow(warnings)]

// mod buildtools;

// #[cfg(feature = "parallel-build")]
// use buildtools::dep_graph::parallel::*;
// use buildtools::dep_graph::{DepGraph, Dependency};
// use buildtools::git::GitRepository;
// use buildtools::libs::{Library, LibraryId, LIBRARIES};
// use buildtools::{feature_env_set, is_debug_build, manifest, output, search};
// #[cfg(feature = "parallel-build")]
// use rayon::prelude::*;
use anyhow::Result;
use dep_graph::{DepGraph, Dependency};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use bindgen::callbacks::{
    EnumVariantCustomBehavior, EnumVariantValue, IntKind, MacroParsingBehavior, ParseCallbacks,
};

#[macro_export]
macro_rules! switch {
    ($conf:expr, $feat:expr, $name:expr) => {
        let arg = if $feat { "enable" } else { "disable" };
        $conf.arg(format!("--{}-{}", arg, $name));
    };
}

pub fn output() -> PathBuf {
    PathBuf::from(std::env::var("OUT_DIR").unwrap())
        .canonicalize()
        .unwrap()
}

pub fn manifest() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .canonicalize()
        .unwrap()
}

pub fn search() -> PathBuf {
    let mut absolute = std::env::current_dir().unwrap();
    absolute.push(&output());
    absolute.push("dist");
    absolute
}

pub fn feature_env_set(name: &str) -> bool {
    std::env::var(&format!("CARGO_FEATURE_FFMPEG_{}", name.to_uppercase())).is_ok()
}

pub fn is_debug_build() -> bool {
    std::env::var("DEBUG").is_ok()
}

pub fn is_cross_build() -> bool {
    || -> Result<bool> {
        let target = std::env::var("TARGET")?;
        let host = std::env::var("HOST")?;
        Ok(target != host)
    }()
    .unwrap_or(false)
}

pub fn build_env() -> HashMap<&'static str, String> {
    let ld_flags = format!("-L{}", search().join("lib").to_string_lossy());
    HashMap::from([
        ("LDFLAGS", ld_flags.clone()),
        (
            "PKG_CONFIG_PATH",
            search().join("lib/pkgconfig").to_string_lossy().to_string(),
        ),
        (
            "CPPFLAGS",
            format!("-I{}", search().join("include").to_string_lossy()),
        ),
        (
            "CFLAGS",
            format!("-I{}", search().join("include").to_string_lossy()),
        ),
    ])
}

pub mod dep_graph {
    pub mod components {
        use super::InnerGraph;
        use std::collections::{HashMap, HashSet};
        use std::hash::{Hash, Hasher};
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, RwLock,
        };

        #[derive(Clone, Debug)]
        struct ComponentNode {
            stack_idx: usize,
            stacked: bool,
        }

        pub struct TarjanStronglyConnectedComponents<'a, I>
        where
            I: Clone + std::fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
        {
            graph: &'a InnerGraph<I>,
            stack: Vec<&'a I>,
            nodes: Vec<ComponentNode>,
            seen: HashMap<&'a I, usize>,
            components: Vec<Vec<&'a I>>,
        }

        impl<'a, I> TarjanStronglyConnectedComponents<'a, I>
        where
            I: Clone + std::fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
        {
            pub fn new(graph: &'a InnerGraph<I>) -> TarjanStronglyConnectedComponents<I> {
                TarjanStronglyConnectedComponents {
                    graph,
                    stack: Vec::<&I>::new(),
                    nodes: Vec::<ComponentNode>::with_capacity(graph.len()),
                    seen: HashMap::<&I, usize>::new(),
                    components: Vec::<Vec<&I>>::new(),
                }
            }

            pub fn has_circles(&'a mut self) -> bool {
                // start depth first search from each node that has not yet been visited
                for node in self.graph.keys() {
                    if !self.seen.contains_key(&node) {
                        self.dfs(&node);
                    }
                }
                // panic!("components: {:?}", self.components);
                self.components.len() != self.graph.len()
            }

            fn dfs(&mut self, node: &'a I) -> &ComponentNode {
                let stack_idx = self.nodes.len();
                self.seen.insert(node, stack_idx);
                self.stack.push(node);
                self.nodes.push(ComponentNode {
                    stack_idx,     // the index of the node on the stack
                    stacked: true, // the node is currently on the stack
                });

                if let Some(links) = self.graph.get(node) {
                    for neighbour in links {
                        match self.seen.get(neighbour) {
                            Some(&i) => {
                                // node was already visited
                                if self.nodes[i].stacked {
                                    self.nodes[stack_idx].stack_idx =
                                        self.nodes[stack_idx].stack_idx.min(i);
                                }
                            }
                            None => {
                                // node has not yet been visited
                                let n = self.dfs(neighbour);
                                let n_stack_idx = n.stack_idx;
                                self.nodes[stack_idx].stack_idx =
                                    self.nodes[stack_idx].stack_idx.min(n_stack_idx);
                            }
                        };
                    }
                }
                // maintain the stack invariant:
                // a node remains on the stack after it has been visited
                // iff there exists a path in the input graph from it some
                // node earlier on the stack
                if self.nodes[stack_idx].stack_idx == stack_idx {
                    let mut circle = Vec::<&I>::new();
                    let mut i = self.stack.len() - 1;
                    loop {
                        let w = self.stack[i];
                        let n_stack_idx = self.seen[w];
                        self.nodes[n_stack_idx].stacked = false;
                        circle.push(w);
                        if n_stack_idx == stack_idx {
                            break;
                        };
                        i -= 1;
                    }
                    self.stack.pop();
                    self.components.push(circle);
                }
                &self.nodes[stack_idx]
            }
        }
    }

    use anyhow::Result;
    use components::TarjanStronglyConnectedComponents;
    use std::cmp::PartialEq;
    use std::collections::{HashMap, HashSet};
    use std::hash::{Hash, Hasher};
    use std::iter::{DoubleEndedIterator, ExactSizeIterator, FromIterator};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    };

    type InnerGraph<I> = HashMap<I, HashSet<I>>;
    type Graph<I> = Arc<RwLock<InnerGraph<I>>>;

    #[derive(Clone, Debug)]
    pub struct Dependency<I>
    where
        I: Clone + std::fmt::Debug + Eq + Hash + PartialEq + Send + Sync,
    {
        id: I,
        deps: HashSet<I>,
    }

    impl<I> Dependency<I>
    where
        I: Clone + std::fmt::Debug + Eq + Hash + PartialEq + Send + Sync,
    {
        pub fn new(id: I) -> Dependency<I> {
            Dependency {
                id,
                deps: HashSet::default(),
            }
        }

        pub fn id(&self) -> &I {
            &self.id
        }
        pub fn deps(&self) -> &HashSet<I> {
            &self.deps
        }
        pub fn add_dep(&mut self, dep: I) {
            self.deps.insert(dep);
        }
    }

    #[derive(Debug)]
    pub struct DepGraph<I>
    where
        I: Clone + std::fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
    {
        pub ready_nodes: HashSet<I>,
        pub deps: Graph<I>,
        pub reverse_deps: Graph<I>,
    }

    impl<I> DepGraph<I>
    where
        I: Clone + std::fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
    {
        pub fn new(nodes: Vec<Dependency<I>>) -> Result<Self> {
            let (ready_nodes, deps, reverse_deps) = DepGraph::parse_nodes(nodes);

            // check for cyclic dependencies
            if TarjanStronglyConnectedComponents::new(&deps).has_circles() {
                panic!("has circles");
            }

            // println!("cargo:warning=deps: {:?}", deps);
            // println!("cargo:warning=reverse deps: {:?}", reverse_deps);

            Ok(DepGraph {
                ready_nodes,
                deps: Arc::new(RwLock::new(deps)),
                reverse_deps: Arc::new(RwLock::new(reverse_deps)),
            })
        }

        /// set of all recursive dependencies for node
        pub fn reacheable(&self, node: &I) -> HashSet<I> {
            let mut seen = HashSet::<I>::new();
            let mut stack = Vec::<I>::new();
            stack.push(node.clone());
            while !stack.is_empty() {
                let cur = stack.pop().unwrap();
                seen.insert(cur.clone());
                if let Some(deps) = self
                    .deps
                    .read()
                    .ok()
                    .as_ref()
                    .and_then(|deps| deps.get(&cur))
                {
                    for dep in deps.iter() {
                        if !seen.contains(dep) {
                            stack.push(dep.clone());
                        }
                    }
                }
            }
            seen
        }

        pub fn shake(&mut self, nodes: Vec<I>) {
            let mut all_reacheable = HashSet::<I>::new();
            for node in nodes {
                all_reacheable.extend(self.reacheable(&node));
            }
            let remove: HashSet<I> = HashSet::from_iter(
                self.deps
                    .read()
                    .unwrap()
                    .keys()
                    .filter(|dep| !all_reacheable.contains(dep))
                    .map(|dep| dep.to_owned()),
            );
            for dep in &remove {
                self.ready_nodes.remove(&dep);
                self.deps.write().unwrap().remove(&dep);
                self.reverse_deps.write().unwrap().remove(&dep);
                for (_, deps) in self.deps.write().unwrap().iter_mut() {
                    deps.remove(dep);
                }
                for (_, deps) in self.reverse_deps.write().unwrap().iter_mut() {
                    deps.remove(dep);
                }
            }
        }

        fn parse_nodes(nodes: Vec<Dependency<I>>) -> (HashSet<I>, InnerGraph<I>, InnerGraph<I>) {
            let mut deps = InnerGraph::<I>::default();
            let mut reverse_deps = InnerGraph::<I>::default();
            let mut ready_nodes = HashSet::<I>::default();

            for node in nodes {
                deps.insert(node.id().clone(), node.deps().clone());

                if node.deps().is_empty() {
                    ready_nodes.insert(node.id().clone());
                }

                for node_dep in node.deps() {
                    if !reverse_deps.contains_key(node_dep) {
                        reverse_deps.insert(
                            node_dep.clone(),
                            HashSet::from_iter(vec![node.id().clone()]),
                        );
                    }

                    // if !reverse_deps.contains_key(node_dep) {
                    //     // let mut dep_reverse_deps = HashSet::new();
                    //     // dep_reverse_deps.insert(node.id().clone());
                    //     reverse_deps.insert(
                    //         node_dep.clone(),
                    //         HashSet::from_iter(vec![node.id().clone()]),
                    //     );
                    //     // dep_reverse_deps.clone());
                    // } else {
                    //     let dep_reverse_deps = reverse_deps.get_mut(node_dep).unwrap();
                    //     dep_reverse_deps.insert(node.id().clone());
                    // }
                    // let dep_reverse_deps = reverse_deps.get_mut(node_dep).unwrap();
                    reverse_deps
                        .get_mut(node_dep)
                        .unwrap()
                        .insert(node.id().clone());
                }
            }

            (ready_nodes, deps, reverse_deps)
        }
    }

    impl<I> IntoIterator for DepGraph<I>
    where
        I: Clone + std::fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
    {
        type Item = I;
        type IntoIter = DepGraphIter<I>;

        fn into_iter(self) -> Self::IntoIter {
            DepGraphIter::<I>::new(
                self.ready_nodes.clone(),
                self.deps.clone(),
                self.reverse_deps,
            )
        }
    }

    #[derive(Clone)]
    pub struct DepGraphIter<I>
    where
        I: Clone + std::fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
    {
        ready_nodes: HashSet<I>,
        deps: Graph<I>,
        reverse_deps: Graph<I>,
    }

    impl<I> DepGraphIter<I>
    where
        I: Clone + std::fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
    {
        pub fn new(ready_nodes: HashSet<I>, deps: Graph<I>, reverse_deps: Graph<I>) -> Self {
            Self {
                ready_nodes,
                deps,
                reverse_deps,
            }
        }
    }

    pub fn remove_node_id<I>(id: I, deps: &Graph<I>, reverse_deps: &Graph<I>) -> Result<Vec<I>>
    where
        I: Clone + std::fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
    {
        let rdep_ids = {
            match reverse_deps.read().unwrap().get(&id) {
                Some(node) => node.clone(),
                // If no node depends on a node, it will not appear
                // in reverse_deps.
                None => Default::default(),
            }
        };

        let mut deps = deps.write().unwrap();
        let next_nodes = rdep_ids
            .iter()
            .filter_map(|rdep_id| {
                let rdep = match deps.get_mut(&rdep_id) {
                    Some(rdep) => rdep,
                    None => return None,
                };

                rdep.remove(&id);

                if rdep.is_empty() {
                    Some(rdep_id.clone())
                } else {
                    None
                }
            })
            .collect();

        // Remove the current node from the list of dependencies.
        deps.remove(&id);

        Ok(next_nodes)
    }

    impl<I> Iterator for DepGraphIter<I>
    where
        I: Clone + std::fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
    {
        type Item = I;

        fn next(&mut self) -> Option<Self::Item> {
            if let Some(id) = self.ready_nodes.iter().next().cloned() {
                self.ready_nodes.remove(&id);
                // remove dependencies and retrieve next available nodes, if any
                let next_nodes =
                    remove_node_id::<I>(id.clone(), &self.deps, &self.reverse_deps).ok()?;
                // push ready nodes
                self.ready_nodes.extend(next_nodes);
                return Some(id);
            }
            None
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum LibraryId {
    FFMPEG,
    MP3LAME,
}

pub trait LibraryFeature {
    fn feature_name(&self) -> Option<&'static str>;
    fn name(&self) -> &'static str;
    fn lib(&self) -> &'static str;

    fn is_enabled(&self) -> bool {
        self.feature_name()
            .map(|name| feature_env_set(name))
            .unwrap_or(true)
    }

    fn exists(&self) -> bool {
        let libs = vec![format!("{}.la", self.lib()), format!("{}.a", self.lib())];
        // println!("cargo:warning={:?}", libs);
        libs.iter()
            .any(|lib| search().join("lib").join(lib).metadata().is_ok())
    }
}

#[derive(Debug, Clone)]
pub struct LibraryDependency {
    pub id: LibraryId,
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub struct LibraryArtifact {
    pub name: &'static str,
    pub lib: &'static str,
    pub ffmpeg_flag: Option<&'static str>,
    pub is_feature: bool,
}

impl LibraryFeature for LibraryArtifact {
    fn feature_name(&self) -> Option<&'static str> {
        if self.is_feature {
            Some(self.name)
        } else {
            None
        }
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn lib(&self) -> &'static str {
        self.lib
    }
}

pub struct Library {
    pub name: &'static str,
    pub version: &'static str,
    pub requires: &'static [LibraryDependency],
    pub artifacts: &'static [LibraryArtifact],
    pub build: Box<dyn Fn(bool, &'static str) -> Result<()> + Send + Sync>,
}

impl std::fmt::Debug for Library {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Library")
            .field("name", &self.name)
            .field("requires", &self.requires)
            .field("artifacts", &self.artifacts)
            .finish()
    }
}

impl Library {
    pub fn needs_rebuild(&self) -> bool {
        self.artifacts.iter().any(|a| a.is_enabled() && !a.exists())
    }
}

pub struct GitRepository<'a> {
    pub url: &'a str,
    pub path: &'a PathBuf,
    pub branch: Option<String>,
}

impl GitRepository<'_> {
    pub fn clone(&self) -> Result<()> {
        let _ = std::fs::remove_dir_all(&self.path);
        let mut cmd = Command::new("git");
        cmd.arg("clone").arg("--depth=1");
        if let Some(branch) = &self.branch {
            cmd.arg("-b").arg(branch);
        }

        cmd.arg(self.url).arg(self.path.to_str().unwrap());
        // println!(
        //     "cargo:warning=Cloning {} into {}",
        //     self.url,
        //     self.path.display()
        // );

        if cmd.status()?.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("fetch failed"))
            // Err(io::Error::new(io::ErrorKind::Other, "fetch failed").into())
        }
    }
}

pub struct CrossBuildConfig {
    prefix: String,
    arch: String,
    target_os: String,
}

impl CrossBuildConfig {
    pub fn guess() -> Option<CrossBuildConfig> {
        if is_cross_build() {
            // Rust targets are subtly different than naming scheme for compiler prefixes.
            // The cc crate has the messy logic of guessing a working prefix,
            // and this is a messy way of reusing that logic.
            let cc = cc::Build::new();
            let compiler = cc.get_compiler();
            let compiler = compiler.path().file_stem().unwrap().to_str()?;
            let suffix_pos = compiler.rfind('-')?; // cut off "-gcc"
            let prefix = compiler[0..suffix_pos].trim_end_matches("-wr").to_string(); // "wr-c++" compiler
            let arch = std::env::var("CARGO_CFG_TARGET_ARCH").ok()?;
            let target_os = std::env::var("CARGO_CFG_TARGET_OS").ok()?;

            Some(CrossBuildConfig {
                prefix,
                arch,
                target_os,
            })
        } else {
            None
        }
    }
}

pub fn build_mp3lame(rebuild: bool, version: &'static str) -> Result<()> {
    let output_base_path = output();
    let source = output_base_path.join(format!("lame-{}", version));
    if rebuild {
        let repo = GitRepository {
            url: "https://github.com/despoa/LAME",
            path: &source,
            branch: Some(format!("lame3_{}", version)),
        };
        repo.clone()?;

        let configure_path = source.join("configure");
        assert!(configure_path.exists());
        let mut configure = Command::new(&configure_path);
        configure.current_dir(&source);
        configure.arg(format!("--prefix={}", search().to_string_lossy()));

        if let Some(cross) = CrossBuildConfig::guess() {
            configure.arg(format!("--cross-prefix={}-", cross.prefix));
            configure.arg(format!("--arch={}", cross.arch));
            configure.arg(format!("--target_os={}", cross.target_os,));
        }

        if is_debug_build() {
            configure.arg("--enable-debug");
        } else {
            configure.arg("--disable-debug");
        }

        // make it static
        configure.arg("--enable-static");
        configure.arg("--disable-shared");

        // run ./configure
        // println!("cargo:warning={:?}", configure);
        let output = configure
            .envs(&build_env())
            .output()
            // .unwrap()
            .unwrap_or_else(|_| panic!("{:?} failed", configure));

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("configure: {}", stdout);

            anyhow::anyhow!("configure failed: {}", &stderr);
            // return Err(io::Error::new(
            //     io::ErrorKind::Other,
            //     format!(
            //         "configure failed {}",
            //         String::from_utf8_lossy(&output.stderr)
            //     ),
            // )
            // .into());
        }

        // run make
        if !Command::new("make")
            .arg("-j")
            .arg(num_cpus::get().to_string())
            .current_dir(&source)
            .envs(&build_env())
            .status()?
            .success()
        {
            return Err(anyhow::anyhow!("make failed"));
            // return Err(io::Error::new(io::ErrorKind::Other, "make failed").into());
        }

        // run make install
        if !Command::new("make")
            .current_dir(&source)
            .arg("install")
            .envs(&build_env())
            .status()?
            .success()
        {
            return Err(anyhow::anyhow!("make install failed"));
            // return Err(io::Error::new(io::ErrorKind::Other, "make install failed").into());
        }
    }

    Ok(())
}

pub fn build_ffmpeg(rebuild: bool, version: &'static str) -> Result<()> {
    let output_base_path = output();
    let source = output_base_path.join(format!("ffmpeg-{}", version));
    if rebuild {
        let repo = GitRepository {
            url: "https://github.com/FFmpeg/FFmpeg",
            path: &source,
            branch: Some(format!("release/{}", version)),
        };
        repo.clone()?;

        let configure_path = source.join("configure");
        assert!(configure_path.exists());
        let mut configure = Command::new(&configure_path);
        configure.current_dir(&source);
        configure.arg(format!("--prefix={}", search().to_string_lossy()));

        let build_envs = build_env();
        configure.arg(format!("--extra-ldflags=\"{}\"", build_envs["LDFLAGS"]));
        configure.arg(format!("--extra-cflags=\"{}\"", build_envs["CFLAGS"]));
        configure.arg("--extra-libs=\"-ldl -lpthread -lm -lz\"");

        if let Some(cross) = CrossBuildConfig::guess() {
            configure.arg(format!("--cross-prefix={}", cross.prefix));
            configure.arg(format!("--arch={}", cross.arch));
            configure.arg(format!("--target_os={}", cross.target_os));
        }

        if is_debug_build() {
            configure.arg("--enable-debug");
            configure.arg("--disable-stripping");
        } else {
            configure.arg("--disable-debug");
            configure.arg("--enable-stripping");
        }

        // make it static
        configure.arg("--pkg-config-flags=\"--static\"");
        configure.arg("--enable-static");
        configure.arg("--disable-shared");
        if cfg!(target_os = "linux") {
            configure.arg("--extra-ldexeflags=\"-static\"");
        }

        // configure.arg("--enable-pic");

        // disable all features and only used what is explicitely enabled
        // configure.arg("--disable-everything");

        // stop autodetected libraries enabling themselves, causing linking errors
        configure.arg("--disable-autodetect");

        // do not build programs since we don't need them
        configure.arg("--disable-programs");

        configure.arg("--disable-network");

        configure.arg("--enable-small");

        // the binary must comply with GPL
        switch!(configure, feature_env_set("LICENSE_GPL"), "gpl");

        // the binary must comply with (L)GPLv3
        switch!(configure, feature_env_set("LICENSE_VERSION3"), "version3");

        // the binary cannot be redistributed
        switch!(configure, feature_env_set("LICENSE_NONFREE"), "nonfree");

        for (_, dep) in LIBRARIES.iter() {
            for feat in dep.artifacts.iter() {
                // if !feat.is_enabled() {
                //     continue;
                // }
                if let Some(flag) = feat.ffmpeg_flag {
                    switch!(configure, feat.is_enabled(), flag);
                }
                // println!("cargo:rustc-link-lib=static={}", feat.name);
                // println!("cargo:warning={}", feat.name);
            }
        }

        // configure external SSL libraries
        // enable!(configure, "FFMPEG_GNUTLS", "gnutls");
        // enable!(configure, "FFMPEG_OPENSSL", "openssl");

        // configure external filters
        // enable!(configure, "FFMPEG_FONTCONFIG", "fontconfig");
        // enable!(configure, "FFMPEG_FREI0R", "frei0r");
        // enable!(configure, "FFMPEG_LADSPA", "ladspa");
        // enable!(configure, "FFMPEG_ASS", "libass");
        // enable!(configure, "FFMPEG_FREETYPE", "libfreetype");
        // enable!(configure, "FFMPEG_FRIBIDI", "libfribidi");
        // enable!(configure, "FFMPEG_OPENCV", "libopencv");
        // enable!(configure, "FFMPEG_VMAF", "libvmaf");

        // configure external encoders/decoders
        // enable!(configure, "FFMPEG_AACPLUS", "libaacplus");
        // enable!(configure, "FFMPEG_CELT", "libcelt");
        // enable!(configure, "FFMPEG_DCADEC", "libdcadec");
        // enable!(configure, "FFMPEG_DAV1D", "libdav1d");
        // enable!(configure, "FFMPEG_FAAC", "libfaac");
        // enable!(configure, "FFMPEG_FDK_AAC", "libfdk-aac");
        // enable!(configure, "FFMPEG_GSM", "libgsm");
        // enable!(configure, "FFMPEG_ILBC", "libilbc");
        // enable!(configure, "FFMPEG_VAZAAR", "libvazaar");
        // enable!(configure, "FFMPEG_MP3LAME", "libmp3lame");
        // enable!(configure, "FFMPEG_OPENCORE_AMRNB", "libopencore-amrnb");
        // enable!(configure, "FFMPEG_OPENCORE_AMRWB", "libopencore-amrwb");
        // enable!(configure, "FFMPEG_OPENH264", "libopenh264");
        // enable!(configure, "FFMPEG_OPENH265", "libopenh265");
        // enable!(configure, "FFMPEG_OPENJPEG", "libopenjpeg");
        // enable!(configure, "FFMPEG_OPUS", "libopus");
        // enable!(configure, "FFMPEG_SCHROEDINGER", "libschroedinger");
        // enable!(configure, "FFMPEG_SHINE", "libshine");
        // enable!(configure, "FFMPEG_SNAPPY", "libsnappy");
        // enable!(configure, "FFMPEG_SPEEX", "libspeex");
        // enable!(configure, "FFMPEG_STAGEFRIGHT_H264", "libstagefright-h264");
        // enable!(configure, "FFMPEG_THEORA", "libtheora");
        // enable!(configure, "FFMPEG_TWOLAME", "libtwolame");
        // enable!(configure, "FFMPEG_UTVIDEO", "libutvideo");
        // enable!(configure, "FFMPEG_VO_AACENC", "libvo-aacenc");
        // enable!(configure, "FFMPEG_VO_AMRWBENC", "libvo-amrwbenc");
        // enable!(configure, "FFMPEG_VORBIS", "libvorbis");
        // enable!(configure, "FFMPEG_VPX", "libvpx");
        // enable!(configure, "FFMPEG_WAVPACK", "libwavpack");
        // enable!(configure, "FFMPEG_WEBP", "libwebp");
        // enable!(configure, "FFMPEG_X264", "libx264");
        // enable!(configure, "FFMPEG_X265", "libx265");
        // enable!(configure, "FFMPEG_AVS", "libavs");
        // enable!(configure, "FFMPEG_XVID", "libxvid");

        // other external libraries
        // enable!(configure, "FFMPEG_DRM", "libdrm");
        // enable!(configure, "FFMPEG_NVENC", "nvenc");

        // configure external protocols
        // enable!(configure, "FFMPEG_SMBCLIENT", "libsmbclient");
        // enable!(configure, "FFMPEG_SSH", "libssh");

        // configure misc build options
        // enable!(configure, "FFMPEG_PIC", "pic");

        // run ./configure
        // let cmd_str: Vec<_> = configure.get_args().collect();
        // let cmd_str = cmd_str
        //     .into_iter()
        //     .map(|arg| arg.to_owned().into_string().unwrap())
        //     .collect::<Vec<String>>()
        //     .join(" ");
        // println!("cargo:warning={:?}", build_env());
        // println!("cargo:warning={:?}", configure);
        // println!("cargo:warning={}", cmd_str);

        configure.envs(&build_envs);

        let result = configure.output().unwrap();
        if !result.status.success() {
            return Err(anyhow::anyhow!("configure failed"));
            // return Err(std::io::Error::new(std::io::ErrorKind::Other, "configure failed").into());
        }

        // .unwrap_or_else(|_| panic!("{:?} failed", configure));
        // if !output.status.success() {
        //     println!("configure: {}", String::from_utf8_lossy(&output.stdout));

        //     return Err(io::Error::new(
        //         io::ErrorKind::Other,
        //         format!(
        //             "configure failed {}",
        //             String::from_utf8_lossy(&output.stderr)
        //         ),
        //     )
        //     .into());
        // }

        // run make
        if !Command::new("make")
            .arg("-j")
            .arg(num_cpus::get().to_string())
            .current_dir(&source)
            .envs(&build_env())
            .status()?
            .success()
        {
            return Err(anyhow::anyhow!("make failed"));
            // return Err(io::Error::new(io::ErrorKind::Other, "make failed").into());
        }

        // run make install
        if !Command::new("make")
            .current_dir(&source)
            .arg("install")
            .envs(&build_env())
            .status()?
            .success()
        {
            return Err(anyhow::anyhow!("make install failed"));
            // return Err(io::Error::new(io::ErrorKind::Other, "make install failed").into());
        }
    }

    for (_, dep) in LIBRARIES.iter() {
        for feat in dep.artifacts.iter() {
            if !feat.is_enabled() {
                continue;
            }
            println!("cargo:rustc-link-lib=static={}", feat.name);
            // println!("cargo:warning={}", feat.name);
        }
    }

    if cfg!(target_os = "macos") {
        let frameworks = vec![
            "AppKit",
            "AudioToolbox",
            "AVFoundation",
            "CoreFoundation",
            "CoreGraphics",
            "CoreMedia",
            "CoreServices",
            "CoreVideo",
            "Foundation",
            "OpenCL",
            "OpenGL",
            "QTKit",
            "QuartzCore",
            "Security",
            "VideoDecodeAcceleration",
            "VideoToolbox",
        ];
        for f in frameworks {
            println!("cargo:rustc-link-lib=framework={}", f);
        }
    }

    // Check additional required libraries.
    {
        use std::io::{BufRead, BufReader};
        let config_mak_path = source.join("ffbuild/config.mak");
        let config_mak_file = std::fs::OpenOptions::new()
            .read(true)
            .open(config_mak_path)
            .unwrap();
        let reader = BufReader::new(config_mak_file);
        let extra_libs = reader
            .lines()
            .find(|line| line.as_ref().unwrap().starts_with("EXTRALIBS"))
            .map(|line| line.unwrap())
            .unwrap();

        // TODO: could use regex here
        let linker_args = extra_libs.split('=').last().unwrap().split(' ');
        let include_libs = linker_args
            .filter(|v| v.starts_with("-l"))
            .map(|flag| &flag[2..]);

        for lib in include_libs {
            println!("cargo:rustc-link-lib={}", lib);
        }
    }

    Ok(())
}
lazy_static::lazy_static! {
    pub static ref LIBRARIES: HashMap<LibraryId, Library> = HashMap::from([
        (
            LibraryId::MP3LAME,
            Library {
                name: "mp3lame",
                version: "99",
                requires: &[],
                build: Box::new(build_mp3lame),
                artifacts: &[LibraryArtifact {
                    name: "mp3lame",
                    lib: "libmp3lame",
                    ffmpeg_flag: Some("libmp3lame"),
                    is_feature: true,
                }],
            },
        ),
        (
            LibraryId::FFMPEG,
            Library {
                name: "ffmpeg",
                version: "4.4",
                // version: "5.0",
                // version: "n4.4.1",
                requires: &[
                    // todo: add the minimal ffmpeg dependencies here
                    // optional dependencies
                    LibraryDependency {
                        optional: true,
                        id: LibraryId::MP3LAME,
                    },
                ],
                build: Box::new(build_ffmpeg),
                artifacts: &[
                    LibraryArtifact {
                        name: "avcodec",
                        lib: "libavcodec",
                        ffmpeg_flag: Some("avcodec"),
                        is_feature: true,
                    },
                    LibraryArtifact {
                        name: "avdevice",
                        lib: "libavdevice",
                        ffmpeg_flag: Some("avdevice"),
                        is_feature: true,
                    },
                    LibraryArtifact {
                        name: "avfilter",
                        lib: "libavfilter",
                        ffmpeg_flag: Some("avfilter"),
                        is_feature: true,
                    },
                    LibraryArtifact {
                        name: "avformat",
                        lib: "libavformat",
                        ffmpeg_flag: Some("avformat"),
                        is_feature: true,
                    },
                    LibraryArtifact {
                        name: "avresample",
                        lib: "libavresample",
                        ffmpeg_flag: Some("avresample"),
                        is_feature: true,
                    },
                    LibraryArtifact {
                        name: "avutil",
                        lib: "libavutil",
                        ffmpeg_flag: Some("avutil"),
                        is_feature: false,
                    },
                    LibraryArtifact {
                        name: "postproc",
                        lib: "libpostproc",
                        ffmpeg_flag: Some("postproc"),
                        is_feature: true,
                    },
                    LibraryArtifact {
                        name: "swresample",
                        lib: "libswresample",
                        ffmpeg_flag: Some("swresample"),
                        is_feature: true,
                    },
                    LibraryArtifact {
                        name: "swscale",
                        lib: "libswscale",
                        ffmpeg_flag: Some("swscale"),
                        is_feature: true,
                    },
                ],
            },
        ),
    ]);
}

#[derive(Debug)]
struct Callbacks;

impl ParseCallbacks for Callbacks {
    fn int_macro(&self, _name: &str, value: i64) -> Option<IntKind> {
        let ch_layout_prefix = "AV_CH_";
        let codec_cap_prefix = "AV_CODEC_CAP_";
        let codec_flag_prefix = "AV_CODEC_FLAG_";
        let error_max_size = "AV_ERROR_MAX_STRING_SIZE";

        if value >= i64::min_value() as i64
            && value <= i64::max_value() as i64
            && _name.starts_with(ch_layout_prefix)
        {
            Some(IntKind::ULongLong)
        } else if value >= i32::min_value() as i64
            && value <= i32::max_value() as i64
            && (_name.starts_with(codec_cap_prefix) || _name.starts_with(codec_flag_prefix))
        {
            Some(IntKind::UInt)
        } else if _name == error_max_size {
            Some(IntKind::Custom {
                name: "usize",
                is_signed: false,
            })
        } else if value >= i32::min_value() as i64 && value <= i32::max_value() as i64 {
            Some(IntKind::Int)
        } else {
            None
        }
    }

    fn enum_variant_behavior(
        &self,
        _enum_name: Option<&str>,
        original_variant_name: &str,
        _variant_value: EnumVariantValue,
    ) -> Option<EnumVariantCustomBehavior> {
        let dummy_codec_id_prefix = "AV_CODEC_ID_FIRST_";
        if original_variant_name.starts_with(dummy_codec_id_prefix) {
            Some(EnumVariantCustomBehavior::Constify)
        } else {
            None
        }
    }

    // https://github.com/rust-lang/rust-bindgen/issues/687#issuecomment-388277405
    fn will_parse_macro(&self, name: &str) -> MacroParsingBehavior {
        use MacroParsingBehavior::*;

        match name {
            "FP_INFINITE" => Ignore,
            "FP_NAN" => Ignore,
            "FP_NORMAL" => Ignore,
            "FP_SUBNORMAL" => Ignore,
            "FP_ZERO" => Ignore,
            _ => Default,
        }
    }
}

#[cfg(not(target_env = "msvc"))]
fn try_vcpkg(_statik: bool) -> Option<Vec<PathBuf>> {
    None
}

#[cfg(target_env = "msvc")]
fn try_vcpkg(statik: bool) -> Option<Vec<PathBuf>> {
    vcpkg::find_package("ffmpeg")
        .map_err(|e| {
            println!("Could not find ffmpeg with vcpkg: {}", e);
        })
        .map(|library| library.include_paths)
        .ok()
}

fn check_features(
    include_paths: Vec<PathBuf>,
    infos: &[(&'static str, Option<&'static str>, &'static str)],
) {
    let mut includes_code = String::new();
    let mut main_code = String::new();
    let infos: Vec<_> = infos
        .iter()
        .filter(|(_, feature, _)| feature.map(|feat| feature_env_set(feat)).unwrap_or(true))
        .collect();

    for &(header, feature, var) in &infos {
        let include = format!("#include <{}>", header);
        if !includes_code.contains(&include) {
            includes_code.push_str(&include);
            includes_code.push('\n');
        }
        includes_code.push_str(&format!(
            r#"
            #ifndef {var}_is_defined
            #ifndef {var}
            #define {var} 0
            #define {var}_is_defined 0
            #else
            #define {var}_is_defined 1
            #endif
            #endif
        "#,
            var = var
        ));

        main_code.push_str(&format!(
            r#"printf("[{var}]%d%d\n", {var}, {var}_is_defined);
            "#,
            var = var
        ));
    }

    let out_dir = output();
    let check_file_path = out_dir.join("check.c");
    let mut check_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(check_file_path)
        .unwrap();

    write!(
        check_file,
        r#"
            #include <stdio.h>
            {includes_code}
            int main()
            {{
                {main_code}
                return 0;
            }}
           "#,
        includes_code = includes_code,
        main_code = main_code
    )
    .unwrap();

    let executable = out_dir.join(if cfg!(windows) { "check.exe" } else { "check" });
    let mut compiler = cc::Build::new()
        // don't cross-compile this
        .target(&std::env::var("HOST").unwrap())
        .get_compiler()
        .to_command();

    for dir in include_paths {
        compiler.arg("-I");
        compiler.arg(dir.to_string_lossy().into_owned());
    }
    if !compiler
        .current_dir(&out_dir)
        .arg("-o")
        .arg(&executable)
        .arg("check.c")
        .status()
        .unwrap()
        .success()
    {
        panic!("Compile failed");
    }

    let check_output = Command::new(out_dir.join(&executable))
        .current_dir(&out_dir)
        .output()
        .unwrap();
    if !check_output.status.success() {
        panic!(
            "{} failed: {}\n{}",
            executable.display(),
            String::from_utf8_lossy(&check_output.stdout),
            String::from_utf8_lossy(&check_output.stderr)
        );
    }

    let stdout = String::from_utf8_lossy(&check_output.stdout).to_string();

    // println!(
    //     "cargo:warning=stdout of {}={}",
    //     executable.display(),
    //     stdout
    // );

    for &(_, feature, var) in &infos {
        let var_str = format!("[{var}]", var = var);
        let pos = var_str.len()
            + stdout
                .find(&var_str)
                .unwrap_or_else(|| panic!("Variable '{}' not found in stdout output", var_str));
        if &stdout[pos..pos + 1] == "1" {
            println!(r#"cargo:rustc-cfg=feature="{}""#, var.to_lowercase());
            println!(r#"cargo:{}=true"#, var.to_lowercase());
        }

        // Also find out if defined or not (useful for cases where only the definition of a macro
        // can be used as distinction)
        if &stdout[pos + 1..pos + 2] == "1" {
            println!(
                r#"cargo:rustc-cfg=feature="{}_is_defined""#,
                var.to_lowercase()
            );
            println!(r#"cargo:{}_is_defined=true"#, var.to_lowercase());
        }
    }
}

fn search_include(include_paths: &[PathBuf], header: &str) -> String {
    for dir in include_paths {
        let include = dir.join(header);
        if std::fs::metadata(&include).is_ok() {
            return include.as_path().to_str().unwrap().to_string();
        }
    }
    format!("/usr/include/{}", header)
}

fn maybe_search_include(include_paths: &[PathBuf], header: &str) -> Option<String> {
    let path = search_include(include_paths, header);
    if std::fs::metadata(&path).is_ok() {
        Some(path)
    } else {
        None
    }
}

fn main() {
    let start = Instant::now();

    if is_debug_build() {
        // println!("cargo:warning=is debug build");
        println!(r#"cargo:rustc-cfg=feature="debug""#);
    }

    let need_build = LIBRARIES.values().any(|lib| lib.needs_rebuild());

    // println!("cargo:warning=need rebuild: {:?}", need_build);
    let mut dependencies = DepGraph::new(
        LIBRARIES
            .iter()
            .filter_map(|(id, lib)| {
                let mut dep = Dependency::new(id.clone());
                for subdep in lib.requires {
                    if !subdep.optional || feature_env_set(LIBRARIES[&subdep.id].name) {
                        dep.add_dep(subdep.id.clone());
                    }
                }
                Some(dep)
            })
            .collect(),
    )
    .unwrap();
    dependencies.shake(vec![LibraryId::FFMPEG]);

    println!(
        "cargo:rustc-link-search=native={}",
        search().join("lib").to_string_lossy()
    );

    if need_build || feature_env_set("force-build") {
        let _ = std::fs::remove_dir_all(&search());
    }

    for inner in dependencies.into_iter() {
        let lib = LIBRARIES.get(&inner).unwrap();
        (lib.build)(need_build, lib.version).unwrap();
    }

    // dependencies.into_par_iter().for_each(|dep| {
    //     let inner = dep.deref();
    //     println!("cargo:warning={:?}", inner);
    //     let lib = LIBRARIES.get(&inner).unwrap();
    //     (lib.build)(need_build, lib.version).unwrap();
    // });

    // make sure the need_build flag works
    assert!(!LIBRARIES.values().any(|lib| lib.needs_rebuild()));

    let include_paths = vec![search().join("include")];
    // let include_paths: Vec<PathBuf> = {
    // let enabled_libraries: Vec<_> = LIBRARIES
    //     .iter()
    //     .filter(|lib| {
    //         !lib.is_feature || lib.feature_name().and_then(|f| env::var(&f).ok()).is_some()
    //     })
    //     .collect();

    // for lib in LIBRARIES[&LibraryId::FFMPEG].artifacts {
    //     println!("cargo:rustc-link-lib=static={}", lib.name);
    // }
    // for lib in &enabled_libraries {
    //     println!("cargo:rustc-link-lib=static={}", lib.name);
    // }
    // if env::var("CARGO_FEATURE_BUILD_ZLIB").is_ok() && cfg!(target_os = "linux") {
    //     println!("cargo:rustc-link-lib=z");
    // }

    // let needs_rebuild = enabled_libraries
    //     .iter()
    //     .map(|lib| search().join("lib").join(format!("lib{}.a", lib.name)))
    //     .any(|lib| lib.metadata().is_err());

    // if false || needs_rebuild {
    //     fs::create_dir_all(&output()).expect("failed to create build directory");
    //     fetch().unwrap();
    //     build_ffmpeg().unwrap();
    // }

    // vec![search().join("include")]
    // };
    // else if let Some(paths) = try_vcpkg(statik) {
    //     // vcpkg doesn't detect the "system" dependencies
    //     if statik {
    //         if cfg!(feature = "avcodec") || cfg!(feature = "avdevice") {
    //             println!("cargo:rustc-link-lib=ole32");
    //         }

    //         if cfg!(feature = "avformat") {
    //             println!("cargo:rustc-link-lib=secur32");
    //             println!("cargo:rustc-link-lib=ws2_32");
    //         }

    //         // avutil depdendencies
    //         println!("cargo:rustc-link-lib=bcrypt");
    //         println!("cargo:rustc-link-lib=user32");
    //     }

    //     paths
    // }
    ////
    //// Fallback to pkg-config
    //else {
    //    pkg_config::Config::new()
    //        .statik(statik)
    //        .probe("libavutil")
    //        .unwrap();

    //    let libs = vec![
    //        ("libavformat", "AVFORMAT"),
    //        ("libavfilter", "AVFILTER"),
    //        ("libavdevice", "AVDEVICE"),
    //        ("libavresample", "AVRESAMPLE"),
    //        ("libswscale", "SWSCALE"),
    //        ("libswresample", "SWRESAMPLE"),
    //    ];

    //    for (lib_name, env_variable_name) in libs.iter() {
    //        if env::var(format!("CARGO_FEATURE_{}", env_variable_name)).is_ok() {
    //            pkg_config::Config::new()
    //                .statik(statik)
    //                .probe(lib_name)
    //                .unwrap();
    //        }
    //    }

    //    pkg_config::Config::new()
    //        .statik(statik)
    //        .probe("libavcodec")
    //        .unwrap()
    //        .include_paths
    //};

    check_features(
        include_paths.clone(),
        &[
            ("libavutil/avutil.h", None, "FF_API_OLD_AVOPTIONS"),
            ("libavutil/avutil.h", None, "FF_API_PIX_FMT"),
            ("libavutil/avutil.h", None, "FF_API_CONTEXT_SIZE"),
            ("libavutil/avutil.h", None, "FF_API_PIX_FMT_DESC"),
            ("libavutil/avutil.h", None, "FF_API_AV_REVERSE"),
            ("libavutil/avutil.h", None, "FF_API_AUDIOCONVERT"),
            ("libavutil/avutil.h", None, "FF_API_CPU_FLAG_MMX2"),
            ("libavutil/avutil.h", None, "FF_API_LLS_PRIVATE"),
            ("libavutil/avutil.h", None, "FF_API_AVFRAME_LAVC"),
            ("libavutil/avutil.h", None, "FF_API_VDPAU"),
            (
                "libavutil/avutil.h",
                None,
                "FF_API_GET_CHANNEL_LAYOUT_COMPAT",
            ),
            ("libavutil/avutil.h", None, "FF_API_XVMC"),
            ("libavutil/avutil.h", None, "FF_API_OPT_TYPE_METADATA"),
            ("libavutil/avutil.h", None, "FF_API_DLOG"),
            ("libavutil/avutil.h", None, "FF_API_HMAC"),
            ("libavutil/avutil.h", None, "FF_API_VAAPI"),
            ("libavutil/avutil.h", None, "FF_API_PKT_PTS"),
            ("libavutil/avutil.h", None, "FF_API_ERROR_FRAME"),
            ("libavutil/avutil.h", None, "FF_API_FRAME_QP"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_VIMA_DECODER",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_REQUEST_CHANNELS",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_OLD_DECODE_AUDIO",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_OLD_ENCODE_AUDIO",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_OLD_ENCODE_VIDEO",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_CODEC_ID"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_AUDIO_CONVERT",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_AVCODEC_RESAMPLE",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_DEINTERLACE",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_DESTRUCT_PACKET",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_GET_BUFFER"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_MISSING_SAMPLE",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_LOWRES"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_CAP_VDPAU"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_BUFS_VDPAU"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_VOXWARE"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_SET_DIMENSIONS",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_DEBUG_MV"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_AC_VLC"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_OLD_MSMPEG4",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_ASPECT_EXTENDED",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_THREAD_OPAQUE",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_CODEC_PKT"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_ARCH_ALPHA"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_ERROR_RATE"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_QSCALE_TYPE",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_MB_TYPE"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_MAX_BFRAMES",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_NEG_LINESIZES",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_EMU_EDGE"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_ARCH_SH4"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_ARCH_SPARC"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_UNUSED_MEMBERS",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_IDCT_XVIDMMX",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_INPUT_PRESERVED",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_NORMALIZE_AQP",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_GMC"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_MV0"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_CODEC_NAME"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_AFD"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_VISMV"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_DV_FRAME_PROFILE",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_AUDIOENC_DELAY",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_VAAPI_CONTEXT",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_AVCTX_TIMEBASE",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_MPV_OPT"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_STREAM_CODEC_TAG",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_QUANT_BIAS"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_RC_STRATEGY",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_CODED_FRAME",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_MOTION_EST"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_WITHOUT_PREFIX",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_CONVERGENCE_DURATION",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_PRIVATE_OPT",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_CODER_TYPE"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_RTP_CALLBACK",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_STAT_BITS"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_VBV_DELAY"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_SIDEDATA_ONLY_PKT",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_AVPICTURE"),
            (
                "libavformat/avformat.h",
                Some("avformat"),
                "FF_API_LAVF_BITEXACT",
            ),
            (
                "libavformat/avformat.h",
                Some("avformat"),
                "FF_API_LAVF_FRAC",
            ),
            (
                "libavformat/avformat.h",
                Some("avformat"),
                "FF_API_URL_FEOF",
            ),
            (
                "libavformat/avformat.h",
                Some("avformat"),
                "FF_API_PROBESIZE_32",
            ),
            (
                "libavformat/avformat.h",
                Some("avformat"),
                "FF_API_LAVF_AVCTX",
            ),
            (
                "libavformat/avformat.h",
                Some("avformat"),
                "FF_API_OLD_OPEN_CALLBACKS",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_AVFILTERPAD_PUBLIC",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_FOO_COUNT",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_OLD_FILTER_OPTS",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_OLD_FILTER_OPTS_ERROR",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_AVFILTER_OPEN",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_OLD_FILTER_REGISTER",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_OLD_GRAPH_PARSE",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_NOCONST_GET_NAME",
            ),
            (
                "libavresample/avresample.h",
                Some("avresample"),
                "FF_API_RESAMPLE_CLOSE_OPEN",
            ),
            (
                "libswscale/swscale.h",
                Some("swscale"),
                "FF_API_SWS_CPU_CAPS",
            ),
            ("libswscale/swscale.h", Some("swscale"), "FF_API_ARCH_BFIN"),
        ],
    );

    if need_build {
        let clang_includes = include_paths
            .iter()
            .map(|include| format!("-I{}", include.to_string_lossy()));

        let mut builder = bindgen::Builder::default()
            .clang_args(clang_includes)
            .ctypes_prefix("libc")
            // https://github.com/rust-lang/rust-bindgen/issues/550
            .blocklist_type("max_align_t")
            .blocklist_function("_.*")
            // Blocklist functions with u128 in signature.
            // https://github.com/zmwangx/rust-ffmpeg-sys/issues/1
            // https://github.com/rust-lang/rust-bindgen/issues/1549
            .blocklist_function("acoshl")
            .blocklist_function("acosl")
            .blocklist_function("asinhl")
            .blocklist_function("asinl")
            .blocklist_function("atan2l")
            .blocklist_function("atanhl")
            .blocklist_function("atanl")
            .blocklist_function("cbrtl")
            .blocklist_function("ceill")
            .blocklist_function("copysignl")
            .blocklist_function("coshl")
            .blocklist_function("cosl")
            .blocklist_function("dreml")
            .blocklist_function("ecvt_r")
            .blocklist_function("erfcl")
            .blocklist_function("erfl")
            .blocklist_function("exp2l")
            .blocklist_function("expl")
            .blocklist_function("expm1l")
            .blocklist_function("fabsl")
            .blocklist_function("fcvt_r")
            .blocklist_function("fdiml")
            .blocklist_function("finitel")
            .blocklist_function("floorl")
            .blocklist_function("fmal")
            .blocklist_function("fmaxl")
            .blocklist_function("fminl")
            .blocklist_function("fmodl")
            .blocklist_function("frexpl")
            .blocklist_function("gammal")
            .blocklist_function("hypotl")
            .blocklist_function("ilogbl")
            .blocklist_function("isinfl")
            .blocklist_function("isnanl")
            .blocklist_function("j0l")
            .blocklist_function("j1l")
            .blocklist_function("jnl")
            .blocklist_function("ldexpl")
            .blocklist_function("lgammal")
            .blocklist_function("lgammal_r")
            .blocklist_function("llrintl")
            .blocklist_function("llroundl")
            .blocklist_function("log10l")
            .blocklist_function("log1pl")
            .blocklist_function("log2l")
            .blocklist_function("logbl")
            .blocklist_function("logl")
            .blocklist_function("lrintl")
            .blocklist_function("lroundl")
            .blocklist_function("modfl")
            .blocklist_function("nanl")
            .blocklist_function("nearbyintl")
            .blocklist_function("nextafterl")
            .blocklist_function("nexttoward")
            .blocklist_function("nexttowardf")
            .blocklist_function("nexttowardl")
            .blocklist_function("powl")
            .blocklist_function("qecvt")
            .blocklist_function("qecvt_r")
            .blocklist_function("qfcvt")
            .blocklist_function("qfcvt_r")
            .blocklist_function("qgcvt")
            .blocklist_function("remainderl")
            .blocklist_function("remquol")
            .blocklist_function("rintl")
            .blocklist_function("roundl")
            .blocklist_function("scalbl")
            .blocklist_function("scalblnl")
            .blocklist_function("scalbnl")
            .blocklist_function("significandl")
            .blocklist_function("sinhl")
            .blocklist_function("sinl")
            .blocklist_function("sqrtl")
            .blocklist_function("strtold")
            .blocklist_function("tanhl")
            .blocklist_function("tanl")
            .blocklist_function("tgammal")
            .blocklist_function("truncl")
            .blocklist_function("y0l")
            .blocklist_function("y1l")
            .blocklist_function("ynl")
            .opaque_type("__mingw_ldbl_type_t")
            .generate_comments(false)
            .default_enum_style(bindgen::EnumVariation::Rust {
                non_exhaustive: std::env::var("CARGO_FEATURE_NON_EXHAUSTIVE_ENUMS").is_ok(),
            })
            .rustified_enum("*")
            .prepend_enum_name(false)
            .derive_eq(true)
            .size_t_is_usize(true)
            .parse_callbacks(Box::new(Callbacks));

        // The input headers we would like to generate
        // bindings for.
        if feature_env_set("avcodec") {
            // if std::env::var("CARGO_FEATURE_AVCODEC").is_ok() {
            builder = builder
                .header(search_include(&include_paths, "libavcodec/avcodec.h"))
                .header(search_include(&include_paths, "libavcodec/dv_profile.h"))
                .header(search_include(&include_paths, "libavcodec/avfft.h"))
                .header(search_include(&include_paths, "libavcodec/vorbis_parser.h"));
            // if ffmpeg_major_version < 5 {
            builder = builder.header(search_include(&include_paths, "libavcodec/vaapi.h"))
            // }
        }

        if feature_env_set("avdevice") {
            // if std::env::var("CARGO_FEATURE_AVDEVICE").is_ok() {
            builder = builder.header(search_include(&include_paths, "libavdevice/avdevice.h"));
        }

        if feature_env_set("avfilter") {
            // if std::env::var("CARGO_FEATURE_AVFILTER").is_ok() {
            builder = builder
                .header(search_include(&include_paths, "libavfilter/buffersink.h"))
                .header(search_include(&include_paths, "libavfilter/buffersrc.h"))
                .header(search_include(&include_paths, "libavfilter/avfilter.h"));
        }

        if feature_env_set("avformat") {
            // if env::var("CARGO_FEATURE_AVFORMAT").is_ok() {
            builder = builder
                .header(search_include(&include_paths, "libavformat/avformat.h"))
                .header(search_include(&include_paths, "libavformat/avio.h"));
        }

        if feature_env_set("avresample") {
            // if env::var("CARGO_FEATURE_AVRESAMPLE").is_ok() {
            builder = builder.header(search_include(&include_paths, "libavresample/avresample.h"));
        }

        builder = builder
            .header(search_include(&include_paths, "libavutil/adler32.h"))
            .header(search_include(&include_paths, "libavutil/aes.h"))
            .header(search_include(&include_paths, "libavutil/audio_fifo.h"))
            .header(search_include(&include_paths, "libavutil/base64.h"))
            .header(search_include(&include_paths, "libavutil/blowfish.h"))
            .header(search_include(&include_paths, "libavutil/bprint.h"))
            .header(search_include(&include_paths, "libavutil/buffer.h"))
            .header(search_include(&include_paths, "libavutil/camellia.h"))
            .header(search_include(&include_paths, "libavutil/cast5.h"))
            .header(search_include(&include_paths, "libavutil/channel_layout.h"))
            .header(search_include(&include_paths, "libavutil/cpu.h"))
            .header(search_include(&include_paths, "libavutil/crc.h"))
            .header(search_include(&include_paths, "libavutil/dict.h"))
            .header(search_include(&include_paths, "libavutil/display.h"))
            .header(search_include(&include_paths, "libavutil/downmix_info.h"))
            .header(search_include(&include_paths, "libavutil/error.h"))
            .header(search_include(&include_paths, "libavutil/eval.h"))
            .header(search_include(&include_paths, "libavutil/fifo.h"))
            .header(search_include(&include_paths, "libavutil/file.h"))
            .header(search_include(&include_paths, "libavutil/frame.h"))
            .header(search_include(&include_paths, "libavutil/hash.h"))
            .header(search_include(&include_paths, "libavutil/hmac.h"))
            .header(search_include(&include_paths, "libavutil/hwcontext.h"))
            .header(search_include(&include_paths, "libavutil/imgutils.h"))
            .header(search_include(&include_paths, "libavutil/lfg.h"))
            .header(search_include(&include_paths, "libavutil/log.h"))
            .header(search_include(&include_paths, "libavutil/lzo.h"))
            .header(search_include(&include_paths, "libavutil/macros.h"))
            .header(search_include(&include_paths, "libavutil/mathematics.h"))
            .header(search_include(&include_paths, "libavutil/md5.h"))
            .header(search_include(&include_paths, "libavutil/mem.h"))
            .header(search_include(&include_paths, "libavutil/motion_vector.h"))
            .header(search_include(&include_paths, "libavutil/murmur3.h"))
            .header(search_include(&include_paths, "libavutil/opt.h"))
            .header(search_include(&include_paths, "libavutil/parseutils.h"))
            .header(search_include(&include_paths, "libavutil/pixdesc.h"))
            .header(search_include(&include_paths, "libavutil/pixfmt.h"))
            .header(search_include(&include_paths, "libavutil/random_seed.h"))
            .header(search_include(&include_paths, "libavutil/rational.h"))
            .header(search_include(&include_paths, "libavutil/replaygain.h"))
            .header(search_include(&include_paths, "libavutil/ripemd.h"))
            .header(search_include(&include_paths, "libavutil/samplefmt.h"))
            .header(search_include(&include_paths, "libavutil/sha.h"))
            .header(search_include(&include_paths, "libavutil/sha512.h"))
            .header(search_include(&include_paths, "libavutil/stereo3d.h"))
            .header(search_include(&include_paths, "libavutil/avstring.h"))
            .header(search_include(&include_paths, "libavutil/threadmessage.h"))
            .header(search_include(&include_paths, "libavutil/time.h"))
            .header(search_include(&include_paths, "libavutil/timecode.h"))
            .header(search_include(&include_paths, "libavutil/twofish.h"))
            .header(search_include(&include_paths, "libavutil/avutil.h"))
            .header(search_include(&include_paths, "libavutil/xtea.h"));

        if feature_env_set("postproc") {
            // if env::var("CARGO_FEATURE_POSTPROC").is_ok() {
            builder = builder.header(search_include(&include_paths, "libpostproc/postprocess.h"));
        }

        if feature_env_set("swresample") {
            // if env::var("CARGO_FEATURE_SWRESAMPLE").is_ok() {
            builder = builder.header(search_include(&include_paths, "libswresample/swresample.h"));
        }

        if feature_env_set("swscale") {
            // if env::var("CARGO_FEATURE_SWSCALE").is_ok() {
            builder = builder.header(search_include(&include_paths, "libswscale/swscale.h"));
        }

        if let Some(hwcontext_drm_header) =
            maybe_search_include(&include_paths, "libavutil/hwcontext_drm.h")
        {
            builder = builder.header(hwcontext_drm_header);
        }

        let bindings = builder.generate().unwrap();

        bindings
            .write_to_file(output().join("bindings.rs"))
            .unwrap();

        bindings
            .write_to_file(manifest().join("bindings.rs"))
            .unwrap();
    }
    if cfg!(target_os = "macos") {
        // required to make tao (from tauri) link
        println!("cargo:rustc-link-lib=framework=ColorSync");
    }
    #[cfg(all(feature = "proto-build", feature = "parallel-build"))]
    proto_build_thread.join().unwrap();

    println!("cargo:warning=build script took: {:?}", start.elapsed());
}

// println!("cargo:rerun-if-changed=build.rs");
// println!("cargo:rustc-link-lib=framework=OpenAL");
// println!("cargo:rustc-link-lib=framework=Foundation");
// println!("cargo:rustc-link-lib=framework=AudioToolbox");
// println!("cargo:rustc-link-lib=framework=CoreAudio");
// println!("cargo:rustc-link-lib=framework=AVFoundation");
// println!("cargo:rustc-link-lib=framework=CoreVideo");
// println!("cargo:rustc-link-lib=framework=CoreMedia");
// println!("cargo:rustc-link-lib=framework=CoreGraphics");
// println!("cargo:rustc-link-lib=framework=OpenGL");
// println!("cargo:rustc-link-lib=framework=ApplicationServices");
// println!("cargo:rustc-link-lib=framework=CoreFoundation");
// println!("cargo:rustc-link-lib=framework=CoreImage");
// println!("cargo:rustc-link-lib=framework=AppKit");
// println!("cargo:rustc-link-lib=framework=OpenCL");
// println!("cargo:rustc-link-lib=framework=VideoToolbox");
// println!("cargo:rustc-link-lib=framework=CoreServices");
// println!("cargo:rustc-link-lib=framework=CoreText");
// println!("cargo:rustc-link-lib=framework=IOKit");
// println!("cargo:rustc-link-lib=framework=ForceFeedback");
// println!("cargo:rustc-link-lib=framework=GameController");
// println!("cargo:rustc-link-lib=framework=Carbon");
// println!("cargo:rustc-link-lib=framework=Metal");
// println!("cargo:rustc-link-lib=dylib=z");
// println!("cargo:rustc-link-lib=dylib=c++");
// println!("cargo:rustc-link-search=native=ffmpeg-build/lib");
// println!("cargo:rustc-link-lib=static=lzma");
// println!("cargo:rustc-link-lib=static=expat");
// println!("cargo:rustc-link-lib=static=iconv");
// println!("cargo:rustc-link-lib=static=gettextpo");
// println!("cargo:rustc-link-lib=static=png16");
// println!("cargo:rustc-link-lib=static=png");
// println!("cargo:rustc-link-lib=static=yasm");
// println!("cargo:rustc-link-lib=static=bz2");
// println!("cargo:rustc-link-lib=static=udfread");
// println!("cargo:rustc-link-lib=static=bluray");
// println!("cargo:rustc-link-lib=static=freetype");
// println!("cargo:rustc-link-lib=static=fribidi");
// println!("cargo:rustc-link-lib=static=fontconfig");
// println!("cargo:rustc-link-lib=static=harfbuzz");
// println!("cargo:rustc-link-lib=static=ass");
// println!("cargo:rustc-link-lib=static=ssl");
// println!("cargo:rustc-link-lib=static=srt");
// println!("cargo:rustc-link-lib=static=snappy");
// println!("cargo:rustc-link-lib=static=openal");
// println!("cargo:rustc-link-lib=static=opencore-amrwb");
// println!("cargo:rustc-link-lib=static=opencore-amrnb");
// println!("cargo:rustc-link-lib=static=opus");
// println!("cargo:rustc-link-lib=static=ogg");
// println!("cargo:rustc-link-lib=static=crypto");
// println!("cargo:rustc-link-lib=static=theora");
// println!("cargo:rustc-link-lib=static=vorbis");
// println!("cargo:rustc-link-lib=static=vorbisenc");
// println!("cargo:rustc-link-lib=static=vorbisfile");
// println!("cargo:rustc-link-lib=static=mp3lame");
// println!("cargo:rustc-link-lib=static=fdk-aac");
// println!("cargo:rustc-link-lib=static=gsm");
// println!("cargo:rustc-link-lib=static=speex");
// println!("cargo:rustc-link-lib=static=zimg");
// println!("cargo:rustc-link-lib=static=vpx");
// println!("cargo:rustc-link-lib=static=webp");
// println!("cargo:rustc-link-lib=static=webpmux");
// println!("cargo:rustc-link-lib=static=webpdemux");
// println!("cargo:rustc-link-lib=static=openjp2");
// println!("cargo:rustc-link-lib=static=aom");
// println!("cargo:rustc-link-lib=static=dav1d");
// println!("cargo:rustc-link-lib=static=xvidcore");
// println!("cargo:rustc-link-lib=static=openh264");
// println!("cargo:rustc-link-lib=static=x264");
// println!("cargo:rustc-link-lib=static=x265");
// println!("cargo:rustc-link-lib=static=avutil");
// println!("cargo:rustc-link-lib=static=avformat");
// println!("cargo:rustc-link-lib=static=postproc");
// println!("cargo:rustc-link-lib=static=avfilter");
// println!("cargo:rustc-link-lib=static=avdevice");
// println!("cargo:rustc-link-lib=static=swscale");
// println!("cargo:rustc-link-lib=static=swresample");
// println!("cargo:rustc-link-lib=static=avcodec");
