pub(crate) fn get_local_tokens() -> (String, String) {
    let prod_token = "???";
    let sandbox_token = "???";
    (prod_token.parse().unwrap(), sandbox_token.parse().unwrap())
}