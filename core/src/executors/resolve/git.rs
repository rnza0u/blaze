use blaze_common::{error::Result, executor::GitOptions, value::Value};
use url::Url;

use super::{
    git_common::{GitHeadlessResolver, GitResolverContext},
    resolver::{ExecutorResolver, ExecutorSource},
};

pub struct GitResolver<'a> {
    delegate: GitHeadlessResolver<'a>,
}

impl<'a> GitResolver<'a> {
    pub fn new(options: GitOptions, context: GitResolverContext<'a>) -> Self {
        Self {
            delegate: GitHeadlessResolver::new(options, context, |_| {}, |_| {}),
        }
    }
}

impl ExecutorResolver for GitResolver<'_> {
    fn resolve(&self, url: &Url) -> Result<ExecutorSource> {
        self.delegate.resolve(url)
    }

    fn update(&self, url: &Url, state: &Value) -> Result<Option<ExecutorSource>> {
        self.delegate.update(url, state)
    }
}
