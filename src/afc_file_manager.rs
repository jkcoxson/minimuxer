use log::{debug, error};
use rusty_libimobiledevice::services::afc::{AfcClient, AfcFileMode};

use crate::{
    device::{fetch_first_device, test_device_connection},
    Errors, Res,
};

#[swift_bridge::bridge]
mod ffi {
    #[swift_bridge(already_declared, swift_name = "MinimuxerError")]
    enum Errors {}

    extern "Rust" {
        type AfcFileManager;
        #[swift_bridge(associated_to = AfcFileManager)]
        fn remove(path: String) -> Result<(), Errors>;
        #[swift_bridge(swift_name = "createDirectory", associated_to = AfcFileManager)]
        fn create_directory(path: String) -> Result<(), Errors>;
        #[swift_bridge(swift_name = "writeFile", associated_to = AfcFileManager)]
        fn write_file(to: String, bytes: &[u8]) -> Result<(), Errors>;
        #[swift_bridge(swift_name = "copyFileOutsideAfc", associated_to = AfcFileManager)]
        fn copy_file_outside_afc(from: String, to: String) -> Result<(), Errors>;
        #[swift_bridge(swift_name = "contents", associated_to = AfcFileManager)]
        fn contents() -> Vec<RustDirectoryEntry>;

        type RustDirectoryEntry;
        fn path(self: &RustDirectoryEntry) -> String;
        fn parent(self: &RustDirectoryEntry) -> String;
        #[swift_bridge(swift_name = "isFile")]
        fn is_file(self: &RustDirectoryEntry) -> bool;
        fn size(self: &RustDirectoryEntry) -> Option<u32>;
        fn children(self: &RustDirectoryEntry) -> Vec<RustDirectoryEntry>;
    }
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub struct RustDirectoryEntry {
    path: String,
    parent: String,
    is_file: bool,
    size: Option<u32>,
    children: Vec<RustDirectoryEntry>,
}
impl RustDirectoryEntry {
    pub fn path(&self) -> String {
        self.path.clone()
    }
    pub fn parent(&self) -> String {
        self.parent.clone()
    }
    pub fn is_file(&self) -> bool {
        self.is_file
    }
    pub fn size(&self) -> Option<u32> {
        self.size
    }
    pub fn children(&self) -> Vec<RustDirectoryEntry> {
        self.children.clone()
    }
}

pub struct AfcFileManager;
impl AfcFileManager {
    fn client() -> Res<AfcClient<'static>> {
        if !test_device_connection() {
            error!("No device connection");
            return Err(Errors::NoConnection);
        }

        match AfcClient::start_service(&fetch_first_device()?, "minimuxer") {
            Ok(afc) => Ok(afc),
            Err(e) => {
                error!("Couldn't start AFC service: {:?}", e);
                Err(Errors::CreateAfc)
            }
        }
    }

    pub fn remove(path: String) -> Res<()> {
        let client = Self::client()?;

        match client.remove_path_and_contents(&path) {
            Ok(_) => {
                debug!("Removed {path}");
                Ok(())
            }
            Err(e) => {
                error!("Couldn't remove {path}: {e:?}");
                Err(Errors::RwAfc)
            }
        }
    }

    pub fn create_directory(path: String) -> Res<()> {
        let client = Self::client()?;

        match client.make_directory(&path) {
            Ok(_) => {
                debug!("Created directory at {path}");
                Ok(())
            }
            Err(e) => {
                error!("Couldn't create directory at {path}: {e:?}");
                Err(Errors::RwAfc)
            }
        }
    }

    pub fn write_file(to: String, bytes: &[u8]) -> Res<()> {
        let client = Self::client()?;

        let handle = match client.file_open(&to, AfcFileMode::WriteOnly) {
            Ok(c) => {
                debug!("Opened file for writing at {to}");
                c
            }
            Err(e) => {
                error!("Couldn't open file for writing at {to}: {e:?}");
                return Err(Errors::RwAfc);
            }
        };

        match client.file_write(handle, bytes.to_vec()) {
            Ok(_) => debug!("Wrote file to {to}"),
            Err(e) => {
                error!("Couldn't write file to {to}: {e:?}");
                return Err(Errors::RwAfc);
            }
        };

        match client.file_close(handle) {
            Ok(_) => {
                debug!("Closed file at {to}");
                Ok(())
            }
            Err(e) => {
                error!("Couldn't close file at {to}: {e:?}");
                Err(Errors::RwAfc)
            }
        }
    }

    /// Copies the file to another place on the filesystem, MAKE SURE TO
    pub fn copy_file_outside_afc(from: String, to: String) -> Res<()> {
        let client = Self::client()?;
        #[cfg(not(test))]
        let to = to[7..].to_string(); // remove the file:// prefix

        let handle = match client.file_open(&from, AfcFileMode::ReadOnly) {
            Ok(c) => {
                debug!("Opened file for reading at {from}");
                c
            }
            Err(e) => {
                error!("Couldn't open file for reading at {from}: {e:?}");
                return Err(Errors::RwAfc);
            }
        };
        let size = Self::file_size(from.clone())?;
        debug!("File size: {size}");

        let bytes = match client.file_read(handle, size) {
            Ok(c) => {
                debug!("Read file at {from}");
                c
            }
            Err(e) => {
                error!("Couldn't read file at {from}: {e:?}");
                return Err(Errors::RwAfc);
            }
        };

        match client.file_close(handle) {
            Ok(_) => debug!("Closed file at {from}"),
            Err(e) => {
                error!("Couldn't close file at {from}: {e:?}");
                return Err(Errors::RwAfc);
            }
        };

        match std::fs::write(&to, unsafe {
            &*(bytes.as_slice() as *const _ as *const [u8])
        }) {
            Ok(_) => debug!("Wrote file to {to}"),
            Err(e) => {
                error!("Couldn't write file to {to}: {e:?}");
                return Err(Errors::RwAfc);
            }
        }

        Ok(())
    }

    fn file_size(path: String) -> Res<u32> {
        let client = Self::client()?;

        match client.get_file_info(&path) {
            Ok(i) => {
                debug!("Got file info of {path}");
                match i.get("st_size") {
                    Some(s) => Ok(s.parse().unwrap()),
                    None => {
                        error!("Couldn't get file size of {path}");
                        Err(Errors::RwAfc)
                    }
                }
            }
            Err(e) => {
                error!("Couldn't get file info of {path}: {e:?}");
                Err(Errors::RwAfc)
            }
        }
    }

    pub fn contents() -> Vec<RustDirectoryEntry> {
        fn file_info(client: &AfcClient, path: &str) -> (bool, Option<u32>) {
            match client.get_file_info(path) {
                Ok(i) => {
                    debug!("Got file info for {path}");
                    (
                        match i.get("st_ifmt") {
                            Some(s) => s == "S_IFDIR",
                            None => {
                                error!("Couldn't get file type for {path}");
                                false
                            }
                        },
                        i.get("st_size").map(|s| s.parse().unwrap()),
                    )
                }
                Err(e) => {
                    error!("Couldn't get file info for {path}: {e:?}");
                    (false, None)
                }
            }
        }

        fn directory_contents(client: &AfcClient, directory_path: &str) -> Option<Vec<String>> {
            match client.read_directory(directory_path) {
                Ok(c) => {
                    debug!("Got directory contents of {directory_path}");
                    Some(c)
                }
                Err(e) => {
                    error!("Couldn't get directory contents of {directory_path}: {e:?}");
                    None
                }
            }
        }

        fn _contents(
            client: &AfcClient,
            directory_path: &str,
            depth: u8,
        ) -> Vec<RustDirectoryEntry> {
            let mut entries = vec![];
            if let Some(contents) = directory_contents(client, directory_path) {
                for entry in contents {
                    if entry == "." || entry == ".." {
                        continue;
                    }

                    let path = format!("{directory_path}{entry}");
                    let (is_directory, size) = file_info(client, &path);
                    entries.push(RustDirectoryEntry {
                        path: if is_directory {
                            format!("{path}/")
                        } else {
                            path.clone()
                        },
                        parent: directory_path.to_owned(),
                        is_file: !is_directory,
                        size,
                        // make sure it doesn't take a really long time to go through everything
                        // TODO: only do this for some directories or exclude some directories
                        children: if is_directory && depth < 3 {
                            _contents(client, &path, depth + 1)
                        } else {
                            vec![]
                        },
                    });
                }
            }
            entries
        }

        let client = match Self::client() {
            Ok(c) => c,
            Err(_) => return vec![],
        };

        _contents(&client, "/", 0)
    }
}
