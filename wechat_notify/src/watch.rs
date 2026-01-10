use std::{collections::HashMap, time::Duration};

use anyhow::{Ok, Result};
use serde::Deserialize;
use wmi::{FilterValue, WMIConnection};

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceCreationEvent")]
#[serde(rename_all = "PascalCase")]
struct NewProcessEvent {
    target_instance: Process
}

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceDeletionEvent")]  // 改为删除事件
#[serde(rename_all = "PascalCase")]
struct ProcessDeletionEvent {
    target_instance: Process
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_Process")]
#[serde(rename_all = "PascalCase")]
#[allow(unused)]
pub struct Process {
    pub process_id: u32,
    pub name: String,
    pub executable_path: Option<String>,
}

pub enum WatchControl {
    End,
    Continue
}


pub fn watch_start<F>(operate: F) -> Result<()>
where 
    F: Fn(Process) -> Result<WatchControl>
{
    let wmi_con = WMIConnection::new()?; 

    let mut filters = HashMap::<String, FilterValue>::new();

    filters.insert("TargetInstance".to_owned(), FilterValue::is_a::<Process>()?);
    
    let iterator = wmi_con.filtered_notification::<NewProcessEvent>(&filters, Some(Duration::from_secs(1)))?;
    
    for result in iterator {
        let process = result?.target_instance;
        match operate(process)? {
            WatchControl::End => break,
            WatchControl::Continue => continue,
        }
    }

    Ok(())
}

pub fn watch_close<F>(operate: F) -> Result<()>
where 
    F: Fn(Process) -> Result<WatchControl>
{
    let wmi_con = WMIConnection::new()?; 

    let mut filters = HashMap::<String, FilterValue>::new();

    filters.insert("TargetInstance".to_owned(), FilterValue::is_a::<Process>()?);
    
    let iterator = wmi_con.filtered_notification::<ProcessDeletionEvent>(&filters, Some(Duration::from_secs(1)))?;
    
    for result in iterator {
        let process = result?.target_instance;
        match operate(process)? {
            WatchControl::End => break,
            WatchControl::Continue => continue,
        }
    }

    Ok(())
}