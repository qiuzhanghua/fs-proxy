use lazy_static::lazy_static;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::BufReader;
use std::io::prelude::*;
use std::path::PathBuf;

lazy_static! {
    pub static ref EXECUTABLE_DIRECTORY: String = match get_executable_directory() {
        Ok(path) => path.to_string_lossy().into_owned(),
        Err(_) => ".".to_string(),
    };
}

pub fn get_executable_directory() -> anyhow::Result<PathBuf> {
    let exe_path = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("Failed to get current executable path: {}", e))?;

    // 如果是符号链接，尝试获取真实路径
    let canonical_path = if exe_path.is_symlink() {
        fs::canonicalize(&exe_path).unwrap_or_else(|_| exe_path.clone())
    } else {
        exe_path.clone()
    };

    let exe_dir = canonical_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Failed to get parent directory of executable"))?;

    Ok(exe_dir.to_path_buf())
}

pub fn write_to_env_file(env_data: HashMap<String, String>, file_path: &str) -> anyhow::Result<()> {
    // 打开文件，如果不存在则创建，追加模式
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        // .append(true)
        .truncate(true)
        .open(file_path)?;

    // 写入键值对
    for (key, value) in env_data {
        let line = format!("{}={}\n", key, value);
        log::debug!("{}", line);
        file.write_all(line.as_bytes())?;
    }
    Ok(())
}

pub fn write_to_default_env_file(env_data: HashMap<String, String>) -> anyhow::Result<()> {
    let path = PathBuf::from(&*EXECUTABLE_DIRECTORY)
        .join(".env")
        .to_str()
        .unwrap_or(".env")
        .to_string();
    log::debug!("{}", path);
    write_to_env_file(env_data, &path)
}

pub fn read_env_to_hashmap() -> anyhow::Result<HashMap<String, String>> {
    let mut env_map = HashMap::<String, String>::new();

    let env_file = PathBuf::from(&*EXECUTABLE_DIRECTORY)
        .join(".env")
        .to_str()
        .unwrap_or(".env")
        .to_string();
    // println!("{:?}", dotenv().ok());

    if dotenv::from_filename(env_file).is_ok() {
        // 获取所有环境变量
        // println!(".env file exists");
        for (key, value) in env::vars() {
            env_map.insert(key, value);
        }
    }

    Ok(env_map)
}

pub fn parse_env_file() -> anyhow::Result<HashMap<String, String>> {
    let env_file = PathBuf::from(&*EXECUTABLE_DIRECTORY)
        .join(".env")
        .to_str()
        .unwrap_or(".env")
        .to_string();
    let file = File::open(env_file)?;
    let reader = BufReader::new(file);

    Ok(reader
        .lines()
        .map_while(Result::ok)
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            line.find('=').map(|pos| {
                (
                    line[..pos].trim().to_string(),
                    line[pos + 1..]
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string(),
                )
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_executable_directory() {
        match get_executable_directory() {
            Ok(path) => {
                assert!(path.exists(), "可执行文件目录不存在");
                println!("可执行文件目录: {:?}", path);
            }
            Err(e) => panic!("获取可执行文件目录失败: {}", e),
        }
    }

    #[test]
    fn test_write_to_default_env_file() {
        let env_data: HashMap<String, String> = HashMap::new();
        match write_to_default_env_file(env_data) {
            Ok(_) => {
                println!("OK")
            }
            Err(e) => panic!("{:?}", e),
        }
    }

    #[test]
    fn test_parse_env_file() {
        match parse_env_file() {
            Ok(hashmap) => {
                assert_eq!(hashmap, HashMap::new());
            }
            Err(e) => panic!("{}", e),
        }
    }
}
