use std::{collections::HashMap, path::PathBuf};


/// This launcher is an interface between Sherlock and another program. As an example, a pomodoro
/// program is used. It can receive several messages such as "stop", "reset", "start" it. For that,
/// the CoupledLauncher needs to spawn a thread listening to.
/// # Parameters
/// - callback: The program to send and receive messages from
/// - socket: The socket to which the messages are sent
/// - actions: The actions that can be sent
#[derive(Clone, Debug)]
pub struct CoupledLauncher {
    pub callback: String,
    pub socket: PathBuf,
    pub actions: HashMap<String, String>
}
