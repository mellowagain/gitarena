use test_context::test_context;
use crate::common::{git, Harness};

mod common;

#[test_context(Harness)]
#[test]
fn mirror_integration_test(ctx: &mut Harness) {
    git(ctx, &["clone", "--bare", "https://github.com/mellowagain/gitarena.git"]);
    git(ctx, &["push", "--mirror", &format!("")]);

}
