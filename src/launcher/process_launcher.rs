use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use procfs::process::{all_processes, Process};
use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use std::collections::HashMap;

use crate::loader::util::AppData;
use crate::sherlock_error;
use crate::utils::errors::{SherlockError, SherlockErrorType};

#[derive(Clone, Debug)]
pub struct ProcessLauncher {
    pub processes: Vec<AppData>,
}

impl ProcessLauncher {
    pub fn new(prio: f32) -> Option<Self> {
        let processes = Self::get_all_processes(prio)?;
        return Some(Self {
            processes,
        });
    }
    pub fn kill(pid: (i32, i32)) -> Result<(), SherlockError> {
        if pid.0 != pid.1 {
            let child = Pid::from_raw(pid.1);
            kill(child, Signal::SIGKILL).map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::CommandExecutionError(format!("Kill process \"{}\"", child)),
                    e.to_string()
                )
            })?;
        };
        let parent = Pid::from_raw(pid.0);
        kill(parent, Signal::SIGKILL).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::CommandExecutionError(format!("Kill process \"{}\"", parent)),
                e.to_string()
            )
        })
    }
    pub fn get_all_processes(prio: f32) -> Option<Vec<AppData>> {
        match all_processes() {
            Ok(procs) => {
                let user_processes: Vec<Process> = procs
                    .flatten()
                    .par_bridge()
                    .filter_map(|p| match p.uid() {
                        Ok(uid) if uid > 0 => Some(p),
                        _ => None,
                    })
                    .collect();
                let mut process_names: HashMap<i32, String> = user_processes
                    .par_iter()
                    .filter_map(|p| {
                        p.exe().ok().and_then(|path| {
                            path.file_name()?.to_str().map(|n| (p.pid, n.to_string()))
                        })
                    })
                    .collect();

                let stats: Vec<_> = user_processes
                    .par_iter()
                    .filter_map(|p| p.stat().ok())
                    .collect();
                let mut tmp: HashMap<i32, i32> = HashMap::new();
                let collected: Vec<AppData> = stats
                    .into_iter()
                    .rev()
                    .filter_map(|item| {
                        if item.ppid == 1 {
                            let named_id = tmp.get(&item.pid).copied().unwrap_or(item.pid);
                            process_names
                                .remove(&named_id)
                                .and_then(|name| Some(((item.pid, named_id), name)))
                        } else if item.tty_nr != 0 {
                            if let Some(r) = tmp.remove(&item.pid) {
                                tmp.insert(item.ppid, r);
                            } else if tmp.get(&item.ppid).is_none() {
                                tmp.insert(item.ppid, item.pid);
                            }
                            None
                        } else if tmp.get(&item.ppid).is_none() {
                            tmp.insert(item.ppid, item.pid);
                            None
                        } else {
                            None
                        }
                    })
                    .map(|((ppid, pid), name)| {
                        let mut data = AppData::new();
                        data.name = name.clone();
                        data.search_string = name;
                        data.priority = prio;
                        data.exec = Some(format!("{},{}", ppid, pid));
                        data
                    })
                    .collect();
                Some(collected)
            }
            Err(_) => None,
        }
    }
}
