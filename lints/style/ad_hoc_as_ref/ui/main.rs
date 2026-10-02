struct Path;

struct ConfigPath {
    path: Path,
}

type PathRef<'a> = &'a Path;

impl ConfigPath {
    fn as_path(&self) -> &Path {
        &self.path
    }

    fn get_path(&self) -> &Path {
        &self.path
    }

    fn as_alias_path(&self) -> PathRef<'_> {
        &self.path
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
}

impl AsRef<Path> for ConfigPath {
    fn as_ref(&self) -> &Path {
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
