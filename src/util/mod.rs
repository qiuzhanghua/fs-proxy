use std::env;
use std::fs;
use std::path::PathBuf;

use lazy_static::lazy_static;

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
}
