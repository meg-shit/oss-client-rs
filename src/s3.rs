use super::config::parser;
use std::error::Error;
use std::path::{Component, Path};

use aws_sdk_s3::{types, Client, Config, Credentials, Endpoint, Region};

pub fn create_client() -> Client {
    let config = parser(false).unwrap();
    let creds = Credentials::new(
        config.aws_access_key_id,
        config.aws_secret_access_key,
        None,
        None,
        "Static",
    );
    let aws_config = Config::builder()
        .endpoint_resolver(Endpoint::immutable(
            config.endpoint.parse().expect("valid URI"),
        ))
        .region(Region::new(config.region))
        .credentials_provider(creds)
        .build();
    Client::from_conf(aws_config)
}

pub async fn upload_file(src: &str, target: &str) -> Result<(), Box<dyn Error>> {
    let src_path = Path::new(src);
    if !src_path.exists() {
        panic!("{:?}不存在", src)
    }

    let (bucket, mut key) = parse_s3_url(target);
    if key.ends_with("/") {
        key = Path::new(&key)
            .join(src_path.file_name().unwrap())
            .to_string_lossy()
            .to_string();
    }

    let client = create_client();
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(types::ByteStream::from_path(src).await?)
        .send()
        .await?;
    Ok(())
}

fn parse_s3_url(url: &str) -> (String, String) {
    if !url.starts_with("s3:") {
        panic!("s3路径无效");
    }
    let mut components = Path::new(url).components();
    components.next();
    let bucket = components.next().unwrap().as_os_str().to_str().unwrap();
    let key_vec: Vec<_> = url.split(bucket).collect();
    let key = key_vec[1].trim_start_matches("/");
    (bucket.to_string(), key.to_string())
}

#[test]
fn parse_s3_url_test() {
    let (bucket, key) = parse_s3_url("s3://bucket/prefix1/prefix2/aaa.txt");
    assert_eq!(bucket, "bucket".to_string());
    assert_eq!(key, "prefix1/prefix2/aaa.txt".to_string());
}
