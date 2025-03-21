//! 本模块用于快速扫描指定目录下特定类型的音游谱面文件
//!
//! 基本用法：
//! ```rust
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let target_dir = std::env::args().nth(1).expect("请提供要扫描的目录路径");
//!     let target_path = PathBuf::from(&target_dir);
//!
//!     let storage_type = detect_storage_type(&target_path).await;
//!     println!("目录所在存储介质类型: {:?}", storage_type);
//!
//!     let start_time = SystemTime::now();
//!     let handle = scan_directory_recursive(target_path, storage_type).await?;
//!     println!("扫描已启动，正在实时收集结果...");
//!
//!     // 实时处理结果示例
//!     let mut total_files = 0;
//!     loop {
//!         // 等待通知或完成
//!         tokio::select! {
//!             _ = handle.notify.notified() => {
//!                 // 处理所有当前队列中的元素
//!                 while let Some(file_info) = handle.queue.pop() {
//!                     total_files += 1;
//!                     println!(
//!                         "[实时更新] 发现文件: {} (大小: {}字节)",
//!                         file_info.relative_path.display(),
//!                         file_info.content.len()
//!                     );
//!                 }
//!             }
//!             _ = async {
//!                 while !handle.is_completed.load(Ordering::Acquire) {
//!                     tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
//!                 }
//!             } => break
//!         }
//!     }
//!
//!     // 处理剩余元素
//!     while let Some(_file_info) = handle.queue.pop() {
//!         total_files += 1;
//!     }
//!
//!     let stop_time = SystemTime::now();
//!     println!("\n扫描完成，共找到 {} 个文件", total_files);
//!     println!(
//!         "总用时：{}秒",
//!         stop_time.duration_since(start_time).unwrap().as_secs_f32()
//!     );
//!
//!     Ok(())
//! }
//! ```

use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::SystemTime,
};

use crossbeam::queue::SegQueue;
use sha2::{Digest, Sha256};
use sysinfo::{DiskKind, Disks};
use tokio::{
    fs,
    sync::{Notify, Semaphore},
};

/// 需要扫描的目标文件扩展名列表
const TARGET_EXTS: [&str; 5] = ["bms", "bme", "bml", "pms", "bmson"];

/// 存储介质类型枚举
#[derive(Debug, Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
#[allow(dead_code)]
pub enum StorageType {
    SSD,
    HDD,
    Unknown(isize),
    Failed,
}

/// 文件信息封装结构
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FileInfo {
    absolute_path: PathBuf,  // 绝对路径
    relative_path: PathBuf,  // 相对于搜索目录的路径
    content: Arc<Box<[u8]>>, // 文件内容
    sha256: [u8; 32],        // SHA256哈希值
}

/// 扫描结果句柄结构体
#[derive(Debug)]
pub struct ScanHandle {
    /// 实时结果队列（线程安全）
    pub queue: Arc<SegQueue<FileInfo>>,
    /// 新数据到达通知（异步条件变量）
    pub notify: Arc<Notify>,
    /// 扫描完成标记（原子布尔值）
    pub is_completed: Arc<AtomicBool>,
}

/// 扫描函数
pub async fn scan_directory_recursive(
    root: PathBuf,
    storage_type: StorageType,
) -> Result<ScanHandle, std::io::Error> {
    let queue = Arc::new(SegQueue::new());
    let notify = Arc::new(Notify::new());
    let is_completed = Arc::new(AtomicBool::new(false));

    let root_clone = root.clone();
    let queue_clone = queue.clone();
    let notify_clone = notify.clone();
    let is_completed_clone = is_completed.clone();

    tokio::spawn(async move {
        let dir_queue = Arc::new(SegQueue::new());
        let root_clone_2 = root_clone.clone();
        dir_queue.push(root_clone_2);

        let semaphore = Arc::new(Semaphore::new(match storage_type {
            StorageType::SSD => 16,
            StorageType::HDD | StorageType::Unknown(_) => 1,
            StorageType::Failed => 1,
        }));

        let mut handles = vec![];
        let worker_count = 1;

        for _ in 0..worker_count {
            let dir_queue = dir_queue.clone();
            let queue = queue_clone.clone();
            let notify = notify_clone.clone();
            let semaphore = semaphore.clone();
            let root = root_clone.clone();

            handles.push(tokio::spawn(worker_thread(
                dir_queue, queue, notify, semaphore, root,
            )));
        }

        // 等待所有工作线程完成
        for handle in handles {
            match handle.await.expect("scan_directory_recursive: Join Error!") {
                Ok(_) => (),
                Err(_err) => (),
            }
        }

        // 设置完成标记
        is_completed_clone.store(true, Ordering::SeqCst);
        notify_clone.notify_one(); // 发送最终完成通知
    });

    Ok(ScanHandle {
        queue,
        notify,
        is_completed,
    })
}

/// 检测指定路径所在存储介质的类型
async fn detect_storage_type(path: &Path) -> StorageType {
    let canonical_path = tokio::fs::canonicalize(path)
        .await
        .unwrap_or_else(|_| path.to_path_buf());

    Disks::new_with_refreshed_list()
        .iter()
        .filter(|d| {
            canonical_path.starts_with(
                d.mount_point()
                    .canonicalize()
                    .unwrap_or_else(|_| path.to_path_buf()),
            )
        })
        .max_by_key(|d| d.mount_point().components().count())
        .map(|d| match d.kind() {
            DiskKind::HDD => StorageType::HDD,
            DiskKind::SSD => StorageType::SSD,
            DiskKind::Unknown(t) => StorageType::Unknown(t),
        })
        .unwrap_or(StorageType::Failed)
}

/// 处理单个目录的核心逻辑
async fn process_directory(
    dir: &Path,
    _root: &Path, // 新增根目录参数
    semaphore: Arc<Semaphore>,
) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut found_files = Vec::new();
    let mut subdirs = Vec::new();

    let entries_result = {
        let permit = semaphore.acquire().await.unwrap();
        let result = fs::read_dir(dir).await;
        drop(permit);
        result
    };
    let mut entries = match entries_result {
        Ok(e) => e,
        Err(_) => return (vec![], vec![]),
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let entry_type = entry
            .file_type()
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
            .expect("Failed to get entry_type!");
        let path = entry.path();

        if entry_type.is_dir() {
            subdirs.push(path);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if TARGET_EXTS.contains(&ext.to_ascii_lowercase().as_str()) {
                found_files.push(path);
            }
        }
    }

    (found_files, subdirs)
}

/// 新增文件处理函数
async fn process_file(
    path: &Path,
    root: &Path,
    semaphore: Arc<Semaphore>,
) -> Result<FileInfo, std::io::Error> {
    // 计算相对路径
    let relative_path = path
        .strip_prefix(root)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::NotFound, err))?;

    // 异步读取文件内容
    let content = Arc::new({
        let permit = semaphore.acquire().await.unwrap();
        let content = fs::read(&path).await?;
        drop(permit);
        content.into_boxed_slice()
    });

    // 计算SHA256
    let content_a = Arc::clone(&content);
    let compute_sha256 = move || {
        let mut hasher = Sha256::new();
        hasher.update(content_a.as_ref());
        let sha256 = hasher.finalize();
        <[u8; 32]>::from(sha256)
    };
    let sha256 = tokio::task::spawn_blocking(compute_sha256)
        .await
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Interrupted, err))?;

    Ok(FileInfo {
        absolute_path: path.to_path_buf(),
        relative_path: relative_path.to_path_buf(),
        content,
        sha256,
    })
}

// 修改工作者线程逻辑
async fn worker_thread(
    dir_queue: Arc<SegQueue<PathBuf>>,
    queue: Arc<SegQueue<FileInfo>>,
    notify: Arc<Notify>,
    semaphore: Arc<Semaphore>,
    root: PathBuf,
) -> Result<(), std::io::Error> {
    while let Some(dir) = dir_queue.pop() {
        let (files, subdirs) = process_directory(&dir, &root, semaphore.clone()).await;

        // 处理文件
        for file_path in files {
            let Ok(file_info) = process_file(&file_path, &root, semaphore.clone()).await else {
                continue;
            };
            queue.push(file_info);
            notify.notify_one(); // 发送新数据通知
        }

        // 处理子目录
        subdirs
            .into_iter()
            .for_each(|subdir| dir_queue.push(subdir));
    }
    Ok(())
}
