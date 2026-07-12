use proptest::prelude::*;
use rawscope_evidence::BundleRelativePath;

proptest! {
    #[test]
    fn accepted_bundle_paths_never_contain_escape_components(
        components in proptest::collection::vec("[a-z0-9_-]{1,8}", 1..6),
    ) {
        let path = components.join("/");
        let parsed = BundleRelativePath::parse(path).expect("generated components are normalized");
        prop_assert!(!parsed.as_str().split('/').any(|component| component == "." || component == ".."));
        prop_assert!(!parsed.as_str().contains('\\'));
    }
}
