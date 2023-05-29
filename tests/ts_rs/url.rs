use crate::ts::assert_ts;

use url::Url;

#[test]
fn url(){
    assert_ts!(Url, "string")
}