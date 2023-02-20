// Jackson Coxson

use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
pub enum Errors {
    Success,
    NoDevice,
    CreateDebug,
    CreateInstproxy,
    LookupApps,
    FindApp,
    BundlePath,
    MaxPacket,
    WorkingDirectory,
    Argv,
    LaunchSuccess,
    Detach,
    FunctionArgs,
    CreateAfc,
    RwAfc,
    InstallApp,
    UninstallApp,
    CreateMisagent,
    ProfileInstall,
    ProfileRemove,
    NoConnection,
}
