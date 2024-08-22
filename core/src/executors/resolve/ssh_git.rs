use blaze_common::{
    error::Result,
    executor::{GitOptions, SshAuthentication, SshFingerprint, SshTransport},
    value::Value,
};
use git2::{CertificateCheckStatus, Cred};
use url::Url;

use super::{
    git_common::{GitHeadlessResolver, GitResolverContext}, resolver::{ExecutorUpdate, ExecutorResolution}, ExecutorResolver
};

pub struct GitOverSshResolver<'a> {
    delegate: GitHeadlessResolver<'a>,
}

impl<'a> GitOverSshResolver<'a> {
    pub fn new(
        git_options: GitOptions,
        ssh_transport: SshTransport,
        authentication: Option<SshAuthentication>,
        context: GitResolverContext<'a>,
    ) -> Self {
        Self {
            delegate: GitHeadlessResolver::new(
                git_options,
                context,
                move |remote_callbacks| {
                    let maybe_fingerprints = ssh_transport.fingerprints().map(|f| f.to_vec());
                    let insecure = ssh_transport.insecure();
                    remote_callbacks.certificate_check(move |certificate, _| {
                        if insecure {
                            return Ok(CertificateCheckStatus::CertificateOk);
                        }

                        let host_key = match certificate.as_hostkey() {
                            Some(host_key) => host_key,
                            None => return Ok(CertificateCheckStatus::CertificatePassthrough),
                        };

                        if let Some(fingerprints) = &maybe_fingerprints {
                            for fingerprint in fingerprints {
                                let is_match = match fingerprint {
                                    SshFingerprint::Md5(expected) => {
                                        host_key.hash_md5().map(|actual| expected == actual)
                                    }
                                    SshFingerprint::Sha1(expected) => {
                                        host_key.hash_sha1().map(|actual| expected == actual)
                                    }
                                    SshFingerprint::Sha256(expected) => {
                                        host_key.hash_sha256().map(|actual| expected == actual)
                                    }
                                }
                                .unwrap_or(false);
                                if is_match {
                                    return Ok(CertificateCheckStatus::CertificateOk);
                                }
                            }
                        }

                        Ok(CertificateCheckStatus::CertificatePassthrough)
                    });

                    if let Some(authentication) = &authentication {
                        let authentication = authentication.clone();
                        remote_callbacks.credentials(move |_, username_from_url, cred_type| {
                            fn extract_username<'a>(
                                username_from_options: Option<&'a str>,
                                username_from_url: Option<&'a str>,
                            ) -> std::result::Result<&'a str, git2::Error>
                            {
                                username_from_options.or(username_from_url).ok_or_else(|| {
                                    git2::Error::new(
                                        git2::ErrorCode::Auth,
                                        git2::ErrorClass::Ssh,
                                        "no username was provided. you can provide a username directly in the URL, or through the executor options.",
                                    )
                                })
                            }

                            if cred_type.is_username(){
                                return match &authentication {
                                    SshAuthentication::Password { username, .. } | SshAuthentication::PrivateKeyFile {
                                        username, ..
                                    } => Cred::username(extract_username(username.as_deref(), username_from_url)?)
                                }
                            }

                            match &authentication {
                                SshAuthentication::Password { password, username } => {
                                    Cred::userpass_plaintext(
                                        extract_username(
                                            username.as_deref(),
                                            username_from_url,
                                        )?,
                                        password,
                                    )
                                }
                                SshAuthentication::PrivateKeyFile {
                                    username,
                                    key,
                                    passphrase,
                                } => {
                                    Cred::ssh_key(
                                        extract_username(
                                            username.as_ref().map(|u| u.as_str()),
                                            username_from_url,
                                        )?,
                                        None,
                                        key,
                                        passphrase.as_deref(),
                                    )
                                },
                            }
                        });
                    }
                },
                |_| {},
            ),
        }
    }
}

impl ExecutorResolver for GitOverSshResolver<'_> {
    fn resolve(&self, url: &Url) -> Result<ExecutorResolution> {
        self.delegate.resolve(url)
    }

    fn update(&self, url: &Url, state: &Value) -> Result<ExecutorUpdate> {
        self.delegate.update(url, state)
    }
}
