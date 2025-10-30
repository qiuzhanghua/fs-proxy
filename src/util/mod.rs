use dotenv::dotenv;
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

pub fn get_executable_directory() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // 获取当前可执行文件的完整路径
    let exe_path = env::current_exe().map_err(|e| format!("获取可执行文件路径失败: {}", e))?;

    // 如果是符号链接，尝试获取真实路径
    let canonical_path = if exe_path.is_symlink() {
        fs::canonicalize(&exe_path).unwrap_or_else(|_| exe_path.clone())
    } else {
        exe_path.clone()
    };

    // 获取父目录
    let exe_dir = canonical_path
        .parent()
        .ok_or_else(|| "无法获取可执行文件所在目录".to_string())?;

    Ok(exe_dir.to_path_buf())
}

pub fn write_to_env_file(
    env_data: HashMap<String, String>,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 打开文件，如果不存在则创建，追加模式
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;

    // 写入键值对
    for (key, value) in env_data {
        let line = format!("{}={}\n", key, value);
        file.write_all(line.as_bytes())?;
    }
    Ok(())
}

pub fn write_to_default_env_file(
    env_data: HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(&*EXECUTABLE_DIRECTORY)
        .join(".env")
        .to_str()
        .unwrap_or(".env")
        .to_string();
    write_to_env_file(env_data, &path)
}

pub fn read_env_to_hashmap() -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let mut env_map = HashMap::<String, String>::new();

    let env_file = PathBuf::from(&*EXECUTABLE_DIRECTORY)
        .join(".env")
        .to_str()
        .unwrap_or(".env")
        .to_string();
    println!("{:?}", dotenv().ok());
    match dotenv::from_filename(env_file) {
        Ok(env) => {
            // 获取所有环境变量
            println!(".env file exists");
            for (key, value) in env::vars() {
                env_map.insert(key, value);
            }
        }
        Err(e) => {
            return Err(e.into());
        }
    }

    Ok(env_map)
}

fn parse_env_file() -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
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
            Err(e) => panic!("{}", e),
        }
    }

    #[test]
    fn test_read_env_to_hashmap() {
        match parse_env_file() {
            Ok(hashmap) => {
                assert_eq!(hashmap, HashMap::new());
            }
            Err(e) => panic!("{}", e),
        }
    }
}
