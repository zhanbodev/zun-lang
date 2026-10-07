use std::path::{Path, PathBuf};

use super::manifest::ProjectConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleNamespace {
    Relative,
    Project,
    Modules,
    Stdlib,
    Dependency,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedModule {
    pub module_path: String,
    pub file_path: PathBuf,
    pub namespace: ModuleNamespace,
}

#[derive(Debug, Clone)]
pub struct ModuleResolver {
    project_root: PathBuf,
    current_file: Option<PathBuf>,
    manifest: Option<ProjectConfig>,
}

impl ModuleResolver {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            current_file: None,
            manifest: None,
        }
    }

    pub fn with_current_file(mut self, current_file: Option<PathBuf>) -> Self {
        self.current_file = current_file;
        self
    }

    pub fn with_manifest(mut self, manifest: Option<ProjectConfig>) -> Self {
        self.manifest = manifest;
        self
    }

    pub fn candidate_paths(&self, module_path: &str) -> Vec<ResolvedModule> {
        if module_path.starts_with("std.") {
            let relative = module_path.trim_start_matches("std.").replace('.', "/") + ".zun";
            return vec![ResolvedModule {
                module_path: module_path.to_string(),
                file_path: self.project_root.join("stdlib").join(relative),
                namespace: ModuleNamespace::Stdlib,
            }];
        }

        let relative_module = module_path.replace('.', "/") + ".zun";
        let package_mod = module_path.replace('.', "/") + "/mod.zun";
        let mut candidates = Vec::new();

        if let Some(current_file) = &self.current_file
            && let Some(parent) = current_file.parent()
        {
            candidates.push(ResolvedModule {
                module_path: module_path.to_string(),
                file_path: parent.join(&relative_module),
                namespace: ModuleNamespace::Relative,
            });
            // package entry: services/mod.zun (relative)
            candidates.push(ResolvedModule {
                module_path: module_path.to_string(),
                file_path: parent.join(&package_mod),
                namespace: ModuleNamespace::Relative,
            });
        }

        candidates.push(ResolvedModule {
            module_path: module_path.to_string(),
            file_path: self.project_root.join(&relative_module),
            namespace: ModuleNamespace::Project,
        });
        // package entry: project_root/services/mod.zun
        candidates.push(ResolvedModule {
            module_path: module_path.to_string(),
            file_path: self.project_root.join(&package_mod),
            namespace: ModuleNamespace::Project,
        });
        candidates.push(ResolvedModule {
            module_path: module_path.to_string(),
            file_path: self.project_root.join("modules").join(&relative_module),
            namespace: ModuleNamespace::Modules,
        });

        if let Some(manifest) = &self.manifest {
            let dep_name = module_path.split('.').next().unwrap_or_default();
            if manifest.dependencies.contains_key(dep_name) {
                candidates.push(ResolvedModule {
                    module_path: module_path.to_string(),
                    file_path: self.project_root.join("deps").join(&relative_module),
                    namespace: ModuleNamespace::Dependency,
                });
            }
        }

        candidates
    }

    pub fn candidate_directories(&self, module_path: &str) -> Vec<ResolvedModule> {
        if module_path == "std" || module_path.starts_with("std.") {
            let relative = module_path
                .trim_start_matches("std")
                .trim_start_matches('.')
                .replace('.', "/");
            return vec![ResolvedModule {
                module_path: module_path.to_string(),
                file_path: if relative.is_empty() {
                    self.project_root.join("stdlib")
                } else {
                    self.project_root.join("stdlib").join(relative)
                },
                namespace: ModuleNamespace::Stdlib,
            }];
        }

        let relative_module = module_path.replace('.', "/");
        let mut candidates = Vec::new();

        if let Some(current_file) = &self.current_file
            && let Some(parent) = current_file.parent()
        {
            candidates.push(ResolvedModule {
                module_path: module_path.to_string(),
                file_path: parent.join(&relative_module),
                namespace: ModuleNamespace::Relative,
            });
        }

        candidates.push(ResolvedModule {
            module_path: module_path.to_string(),
            file_path: self.project_root.join(&relative_module),
            namespace: ModuleNamespace::Project,
        });
        candidates.push(ResolvedModule {
            module_path: module_path.to_string(),
            file_path: self.project_root.join("modules").join(&relative_module),
            namespace: ModuleNamespace::Modules,
        });

        if let Some(manifest) = &self.manifest {
            let dep_name = module_path.split('.').next().unwrap_or_default();
            if manifest.dependencies.contains_key(dep_name) {
                candidates.push(ResolvedModule {
                    module_path: module_path.to_string(),
                    file_path: self.project_root.join("deps").join(&relative_module),
                    namespace: ModuleNamespace::Dependency,
                });
            }
        }

        candidates
    }

    pub fn resolve_module(&self, module_path: &str) -> Option<ResolvedModule> {
        self.candidate_paths(module_path)
            .into_iter()
            .find(|candidate| candidate.file_path.exists())
    }

    pub fn resolve_module_directory(&self, module_path: &str) -> Option<ResolvedModule> {
        self.candidate_directories(module_path)
            .into_iter()
            .find(|candidate| candidate.file_path.is_dir())
    }

    pub fn describe_search_order(&self, module_path: &str) -> String {
        self.candidate_paths(module_path)
            .into_iter()
            .map(|candidate| candidate.file_path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn describe_directory_search_order(&self, module_path: &str) -> String {
        self.candidate_directories(module_path)
            .into_iter()
            .map(|candidate| candidate.file_path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }
}
