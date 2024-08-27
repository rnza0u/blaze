use blaze_common::{
    error::Result,
    executor::{GitOptions, GitPlainAuthentication, HttpTransport},
    value::Value,
};
use git2::{CertificateCheckStatus, Cred};
use url::Url;

use super::{
    git_common::{GitHeadlessResolver, GitResolverContext},
    resolver::{ExecutorResolution, ExecutorUpdate},
    ExecutorResolver,
};

pub struct GitOverHttpResolver<'a> {
    delegate: GitHeadlessResolver<'a>,
}

impl<'a> GitOverHttpResolver<'a> {
    pub fn new(
        git_options: GitOptions,
        http_transport: HttpTransport,
        authentication: Option<GitPlainAuthentication>,
        context: GitResolverContext<'a>,
    ) -> Self {
        let headers = http_transport.headers().clone();
        let insecure = http_transport.insecure();

        Self {
            delegate: GitHeadlessResolver::new(
                git_options,
                context,
                move |remote_callbacks| {
                    remote_callbacks.certificate_check(move |_certificate, _| {
                        Ok(if insecure {
                            CertificateCheckStatus::CertificateOk
                        } else {
                            CertificateCheckStatus::CertificatePassthrough
                        })
                    });

                    if let Some(authentication) = &authentication {
                        let authentication = authentication.clone();
                        remote_callbacks.credentials(move |_, _, _| {
                            Cred::userpass_plaintext(
                                authentication.username(),
                                authentication.password(),
                            )
                        });
                    }
                },
                move |fetch_options| {
                    fetch_options.follow_redirects(git2::RemoteRedirect::All);
                    if !headers.is_empty() {
                        let formatted_headers = headers
                            .iter()
                            .map(|(name, value)| format!("{name}: {value}"))
                            .collect::<Vec<_>>();
                        fetch_options.custom_headers(
                            formatted_headers
                                .iter()
                                .map(String::as_str)
                                .collect::<Vec<_>>()
                                .as_slice(),
                        );
                    }
                },
            ),
        }
    }
}

impl ExecutorResolver for GitOverHttpResolver<'_> {
    fn resolve(&self, url: &Url) -> Result<ExecutorResolution> {
        self.delegate.resolve(url)
    }

    fn update(&self, url: &Url, state: &Value) -> Result<ExecutorUpdate> {
        self.delegate.update(url, state)
    }
}
