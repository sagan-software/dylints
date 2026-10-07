struct Path;

struct ConfigPath {
    path: Path,
}

type PathRef<'a> = &'a Path;

impl ConfigPath {
    const LABEL: &'static str = "config";

    fn as_path(&self) -> &Path {
        &self.path
    }

    fn get_path(&self) -> &Path {
        &self.path
    }

    fn as_alias_path(&self) -> PathRef<'_> {
        &self.path
    }

    fn as_mut_path(&mut self) -> &mut Path {
        &mut self.path
    }

    fn as_path_box(&self) -> Box<Path> {
        Box::new(Path)
    }

    fn path_for_profile(&self, profile: &str) -> &Path {
        let _ = profile;
        &self.path
    }

    fn get_path_for_profile(&self, profile: &str) -> &Path {
        let _ = profile;
        &self.path
    }

    fn get_shared(other: &Self) -> &Path {
        &other.path
    }

    fn as_owned_path(self) -> Path {
        self.path
    }
}

struct ImplementedPath {
    path: Path,
}

impl ImplementedPath {
    fn as_path(&self) -> &Path {
        &self.path
    }

    fn as_mut_path(&mut self) -> &mut Path {
        &mut self.path
    }
}

impl AsRef<Path> for ImplementedPath {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl AsMut<Path> for ImplementedPath {
    fn as_mut(&mut self) -> &mut Path {
        &mut self.path
    }
}

trait PathView {
    fn as_view(&self) -> &Path;
}

impl PathView for ConfigPath {
    fn as_view(&self) -> &Path {
        &self.path
    }
}

struct LazyPath {
    path: Option<Path>,
}

impl LazyPath {
    fn get_or_initialize_path(&mut self) -> &Path {
        if self.path.is_none() {
            self.path = Some(Path);
        }

        self.path.as_ref().unwrap()
    }
}

fn main() {}
