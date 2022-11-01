// Jackson Coxson

use libc::c_int;

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

impl From<Errors> for u32 {
    fn from(x: Errors) -> Self {
        match x {
            Errors::Success => 0,
            Errors::NoDevice => 1,
            Errors::CreateDebug => 2,
            Errors::CreateInstproxy => 3,
            Errors::LookupApps => 4,
            Errors::FindApp => 5,
            Errors::BundlePath => 6,
            Errors::MaxPacket => 7,
            Errors::WorkingDirectory => 8,
            Errors::Argv => 9,
            Errors::LaunchSuccess => 10,
            Errors::Detach => 11,
            Errors::FunctionArgs => 12,
            Errors::CreateAfc => 13,
            Errors::RwAfc => 14,
            Errors::InstallApp => 15,
            Errors::UninstallApp => 16,
            Errors::CreateMisagent => 17,
            Errors::ProfileInstall => 18,
            Errors::ProfileRemove => 19,
            Errors::NoConnection => 20,
        }
    }
}

impl From<Errors> for c_int {
    fn from(x: Errors) -> Self {
        0 - (u32::from(x) as c_int)
    }
}
