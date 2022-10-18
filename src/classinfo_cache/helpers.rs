use crate::{
    response::classinfo::ClassInfo,
    error::FileError,
    types::{
        ClassInfoClass,
        ClassInfoAppClass,
        AppId,
    },
};
use super::types::ClassInfoFile;
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
    collections::{HashMap, HashSet},
};
use futures::future::join_all;
use async_fs::File;
use tokio::task::JoinHandle;
use futures_lite::io::AsyncWriteExt;

async fn load_classinfo(
    class: ClassInfoClass,
    data_directory: &PathBuf, 
) -> Result<ClassInfoFile, FileError> {
    let filepath = get_classinfo_file_path(&class, false, data_directory);
    let data = async_fs::read_to_string(filepath).await?;
    let classinfo = serde_json::from_str::<ClassInfo>(&data)?;
            
    Ok((class, classinfo))
}

fn get_classinfo_file_path(
    class: &ClassInfoClass,
    is_temp: bool,
    data_directory: &PathBuf, 
) -> PathBuf {
    let (appid, classid, instanceid) = class;
    let instanceid = match instanceid {
        Some(instanceid) => *instanceid,
        None => 0,
    };
    let filename = match is_temp {
        true => {
            let start = SystemTime::now();
            let timestamp = start
                .duration_since(UNIX_EPOCH)
                // In any reasonable setting this shouldn't panic...
                .expect("Invalid system time")
                .as_millis();
                
            format!("{}_{}_{}.json.{}.temp", appid, classid, instanceid, timestamp)
        },
        false => format!("{}_{}_{}.json", appid, classid, instanceid),
    };
    
    data_directory.join(filename)
}

/// Performs a basic atomic file write.
async fn save_classinfo(
    class: ClassInfoClass,
    classinfo: String,
    data_directory: &PathBuf, 
) -> Result<(), FileError> {
    let temp_filepath = get_classinfo_file_path(
        &class,
        true,
        data_directory,
    );
    let mut temp_file = File::create(&temp_filepath).await?;

    match temp_file.write_all(classinfo.as_bytes()).await {
        Ok(_) => {
            let filepath = get_classinfo_file_path(
                &class,
                false,
                data_directory,
            );
            
            temp_file.flush().await?;
            async_fs::rename(temp_filepath, filepath).await?;

            Ok(())
        },
        Err(error) => {
            // something went wrong writing to this file...
            async_fs::remove_file(&temp_filepath).await?;
            
            Err(error.into())
        }
    }
}

pub async fn load_classinfos(
    classes: &HashSet<&ClassInfoClass>,
    data_directory: &PathBuf, 
) -> Vec<Result<ClassInfoFile, FileError>> {
    let mut tasks: Vec<JoinHandle<Result<ClassInfoFile, FileError>>>= vec![];
    
    for class in classes {
        // must be cloned to move across threads
        let class = **class;
        let class_data_directory = data_directory.clone();
        
        tasks.push(tokio::spawn(async move {
            load_classinfo(class, &class_data_directory).await
        }));
    }
    
    let mut results: Vec<Result<ClassInfoFile, FileError>> = Vec::new();
    
    for join_result in join_all(tasks).await {
        results.push(match join_result {
            Ok(task_result) => task_result,
            Err(_err) => Err(FileError::JoinError),
        })
    }
    
    results
}

pub async fn save_classinfos(
    appid: AppId,
    classinfos: &HashMap<ClassInfoAppClass, String>,
    data_directory: &PathBuf, 
) -> Vec<Result<(), FileError>> {
    let mut tasks: Vec<JoinHandle<Result<(), FileError>>>= vec![];
    
    for ((classid, instanceid), classinfo) in classinfos {
        // must be cloned to move across threads
        let classinfo = classinfo.clone();
        let class = (appid, *classid, *instanceid);
        let class_data_directory = data_directory.clone();
        
        tasks.push(tokio::spawn(async move {
            save_classinfo(class, classinfo, &class_data_directory).await
        }));
    }
    
    let mut results: Vec<Result<(), FileError>> = Vec::new();
    
    for join_result in join_all(tasks).await {
        results.push(match join_result {
            Ok(task_result) => task_result,
            Err(_err) => Err(FileError::JoinError),
        })
    }

    results
}