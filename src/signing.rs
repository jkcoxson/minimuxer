// use der::Document;
// use log::{error, info};
// use p12::PFX;

// use crate::{Errors, Res};

// #[swift_bridge::bridge]
// mod ffi {
//     #[swift_bridge(already_declared, swift_name = "MinimuxerError")]
//     enum Errors {}

//     extern "Rust" {
//         fn sign_app(
//             app_path: String,
//             bundle_id: String,
//             cert_name: String,
//             cert: String,
//             key: String,
//         ) -> Result<(), Errors>;
//     }
// }

// pub fn sign_app(
//     app_path: String,
//     bundle_id: String,
//     cert_name: String,
//     cert: String,
//     key: String,
// ) -> Res<()> {
//     #[cfg(not(test))]
//     let app_path = app_path[7..].to_string(); // remove the file:// prefix

//     // convert cert and key from PEM to DER
//     let (_, cert_der) = Document::from_pem(&cert)
//         .map_err(|e| Errors::AppleCodesign(format!("Couldn't convert cert to document: {e:?}",)))?;
//     let cert_der = cert_der.as_bytes();

//     let (_, key_der) = Document::from_pem(&key)
//         .map_err(|e| Errors::AppleCodesign(format!("Couldn't convert key to document: {e:?}",)))?;
//     let key_der = key_der.as_bytes();

//     // ALTCertificate's p12Data function gives a p12 with no private key for some reason,
//     // so we need to take private key and certificate separately and reconstruct the p12
//     // and then deconstruct to give to apple-codesign. Yes, it's slightly inefficient, but it works.
//     let pfx = PFX::new(cert_der, key_der, None, "", &cert_name).ok_or(Errors::AppleCodesign(
//         format!("Couldn't create PFX from certificate and private key"),
//     ))?;
//     let pfx_data = pfx.to_der();

//     info!("Signing {bundle_id} at {app_path}");

//     match apple_codesign_wrapper::sign_app(&app_path, &bundle_id, &pfx_data, "") {
//         Ok(_) => {
//             info!("Successfully signed app!");
//             Ok(())
//         }
//         Err(e) => {
//             error!("Failed to sign app: {e:?}");
//             Err(Errors::AppleCodesign(format!("{e}")))
//         }
//     }
// }
