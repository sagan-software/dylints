fn main() {
    insta::assert_json_snapshot!(vec![1, 2, 3]);
    insta::assert_yaml_snapshot!(vec![1, 2, 3]);
}
