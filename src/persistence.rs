use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::Cache;

#[derive(Clone)]
pub struct Persistence {
    path: PathBuf,
}

impl Persistence {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn append_set(&self, key: &str, value: &str) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        writeln!(file, "SET {key} {value}")?;
        Ok(())
    }

    pub fn append_delete(&self, key: &str) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        writeln!(file, "DELETE {key}")?;
        Ok(())
    }

    pub fn load_cache(&self) -> io::Result<Cache> {
        let mut cache = Cache::new();

        let file = match std::fs::File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(cache);
            }
            Err(error) => return Err(error),
        };

        let reader = BufReader::new(file);

        for (line_number, line) in reader.lines().enumerate() {
            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();

            match parts.as_slice() {
                ["DELETE", key] => {
                    cache.delete(key);
                }
                ["SET", key, value] => {
                    cache.set(*key, *value);
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid log entry on line {}", line_number + 1),
                    ));
                }
            }
        }

        Ok(cache)
    }

    pub fn replay(&self, cache: &mut Cache) -> io::Result<()> {
        let loaded_cache = self.load_cache()?;
        *cache = loaded_cache;
        Ok(())
    }
}
