#![allow(dead_code)]

use aws_sdk_s3::model::{CompletedMultipartUpload, CompletedPart};
use aws_sdk_s3::{Client, Config, Credentials, Endpoint, Region};
use aws_smithy_http::byte_stream::{ByteStream, Length};
use aws_smithy_types::date_time::Format;
use futures::future::join_all;
use oss_client_rs_conf::config;
use std::collections::HashSet;
use std::error::Error;
use std::io::{stdout, Write};
use std::path::Path;
use walkdir::WalkDir;

const MB: u64 = 1024 * 1024;
const GB: u64 = 1024 * MB;
// 默认分片大小为 8MB
const CHUNK_SIZE: u64 = 8 * MB;
// 默认最大分片数量为 10000
const MAX_CHUNKS: u64 = 10000;

pub fn create_client() -> Client {
    let config = config::parser(false).unwrap();
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

pub async fn upload_file(client: &Client, src: &str, target: &str) -> Result<(), Box<dyn Error>> {
    let file_size = tokio::fs::metadata(src)
        .await
        .expect("it exists I swear")
        .len();
    if file_size > CHUNK_SIZE {
        mutl_upload_v2(&client, src, target).await?
    } else {
        upload_object(&client, src, target).await?
    }
    Ok(())
}

pub async fn upload_object(client: &Client, src: &str, target: &str) -> Result<(), Box<dyn Error>> {
    let (bucket, key) = path_deal(src, target);
    client
        .put_object()
        .bucket(&bucket)
        .key(&key)
        .body(ByteStream::from_path(src).await?)
        .send()
        .await?;
    println!("upload {:#} to s3://{:#}/{:#}", src, bucket, key);
    Ok(())
}

pub async fn mutl_upload(client: &Client, src: &str, target: &str) -> Result<(), Box<dyn Error>> {
    let (bucket, key) = path_deal(src, target);

    // 创建分片请求
    let upload_id = client
        .create_multipart_upload()
        .bucket(&bucket)
        .key(&key)
        .send()
        .await?
        .upload_id()
        .unwrap()
        .to_string();

    let file_size = tokio::fs::metadata(src)
        .await
        .expect("it exists I swear")
        .len();

    let mut chunk_count = (file_size / CHUNK_SIZE) + 1;
    let mut size_of_last_chunk = file_size % CHUNK_SIZE;
    if size_of_last_chunk == 0 {
        size_of_last_chunk = CHUNK_SIZE;
        chunk_count -= 1;
    }

    if file_size == 0 {
        panic!("Bad file size.");
    }
    if chunk_count > MAX_CHUNKS {
        panic!("Too many chunks! Try increasing your chunk size.")
    }

    let mut upload_parts: Vec<CompletedPart> = Vec::new();

    for chunk_index in 0..chunk_count {
        let this_chunk = if chunk_count - 1 == chunk_index {
            size_of_last_chunk
        } else {
            CHUNK_SIZE
        };
        let stream = ByteStream::read_from()
            .path(src)
            .offset(chunk_index * CHUNK_SIZE)
            .length(Length::Exact(this_chunk))
            .build()
            .await
            .unwrap();
        //Chunk index needs to start at 0, but part numbers start at 1.
        let part_number = (chunk_index as i32) + 1;
        // snippet-start:[rust.example_code.s3.upload_part]
        let upload_part_res = client
            .upload_part()
            .key(&key)
            .bucket(&bucket)
            .upload_id(&upload_id)
            .body(stream)
            .part_number(part_number)
            .send()
            .await?;
        upload_parts.push(
            CompletedPart::builder()
                .e_tag(upload_part_res.e_tag.unwrap_or_default())
                .part_number(part_number)
                .build(),
        );
    }

    let completed_multipart_upload: CompletedMultipartUpload = CompletedMultipartUpload::builder()
        .set_parts(Some(upload_parts))
        .build();

    let _complete_multipart_upload_res = client
        .complete_multipart_upload()
        .bucket(&bucket)
        .key(&key)
        .multipart_upload(completed_multipart_upload)
        .upload_id(upload_id)
        .send()
        .await?;
    Ok(())
}

pub async fn mutl_upload_v2(
    client: &Client,
    src: &str,
    target: &str,
) -> Result<(), Box<dyn Error>> {
    let (bucket, key) = path_deal(src, target);
    // 列出所有的分片任务
    let list_mult_part_uploads = client
        .list_multipart_uploads()
        .bucket(&bucket)
        .prefix(&key)
        .send()
        .await?;
    // 统计已上传的part和part大小
    let mut upload_parts: Vec<CompletedPart> = Vec::new();
    let mut part_size: u64 = CHUNK_SIZE;
    let mut upload_id = String::from("");
    #[allow(unused_assignments)]
    let mut upload_size: u64 = 0;
    // 查看所有分片任务中的part
    for mult_part_upload in list_mult_part_uploads.uploads().unwrap_or_default().iter() {
        let list_parts = client
            .list_parts()
            .bucket(&bucket)
            .key(&key)
            .upload_id(mult_part_upload.upload_id().unwrap())
            .max_parts(MAX_CHUNKS as i32)
            .send()
            .await?;
        // 将没有part的任务 abort
        if list_parts.parts().is_none() {
            client
                .abort_multipart_upload()
                .bucket(&bucket)
                .key(&key)
                .upload_id(mult_part_upload.upload_id().unwrap())
                .send()
                .await?;
            continue;
        };
        // 统计已上传part和part大小
        for (index, part) in list_parts.parts().unwrap().iter().enumerate() {
            upload_parts.push(
                CompletedPart::builder()
                    .e_tag(part.e_tag().unwrap())
                    .part_number(part.part_number())
                    .build(),
            );
            if index == 0 {
                part_size = part.size() as u64;
            }
        }
        upload_id = mult_part_upload.upload_id().unwrap().to_string();
        // 只统计第一个包含part的分片任务
        break;
    }
    // 未找到multpart任务时创建，并清空upload_parts
    if upload_id.len() == 0 {
        upload_id = client
            .create_multipart_upload()
            .bucket(&bucket)
            .key(&key)
            .send()
            .await?
            .upload_id()
            .unwrap()
            .to_string();
        upload_parts.clear();
    }
    // 获取文件大小
    let file_size = tokio::fs::metadata(src)
        .await
        .expect("获取文件大小失败")
        .len();
    let total_size = get_size_in_nice(file_size);
    // 根据当前part大小确定part数量
    let mut chunk_count = (file_size / part_size) + 1;
    // part数量大于MAX_CHUNKS时需要重新计算并创建新的multpart任务
    if chunk_count > MAX_CHUNKS {
        part_size = (file_size / MAX_CHUNKS) + 1;
        chunk_count = MAX_CHUNKS;
        // 需要放弃之前已上传的part，重新上传
        client
            .abort_multipart_upload()
            .bucket(&bucket)
            .key(&key)
            .upload_id(&upload_id)
            .send()
            .await?;
        // 创建新任务
        upload_id = client
            .create_multipart_upload()
            .bucket(&bucket)
            .key(&key)
            .send()
            .await?
            .upload_id()
            .unwrap()
            .to_string();
        upload_parts.clear();
    };
    // 确定最终part大小和数量
    let mut size_of_last_chunk = file_size % part_size;
    if size_of_last_chunk == 0 {
        size_of_last_chunk = part_size;
        chunk_count -= 1;
    }
    // 上传part并忽略已上传的part
    let mut upload_patrs_num: HashSet<i32> = HashSet::new();
    for part in &upload_parts {
        upload_patrs_num.insert(part.part_number());
    }
    upload_size = upload_patrs_num.len() as u64 * part_size;

    let mut futures = vec![];
    for chunk_index in 0..chunk_count {
        let this_chunk = if chunk_count - 1 == chunk_index {
            size_of_last_chunk
        } else {
            part_size
        };
        let stream = ByteStream::read_from()
            .path(src)
            .offset(chunk_index * part_size)
            .length(Length::Exact(this_chunk))
            .build()
            .await
            .unwrap();
        //Chunk index needs to start at 0, but part numbers start at 1.
        let part_number = (chunk_index as i32) + 1;
        // 跳过已上传的part
        if upload_patrs_num.contains(&part_number) {
            continue;
        }
        // snippet-start:[rust.example_code.s3.upload_part]
        let upload_part_res = client
            .upload_part()
            .key(&key)
            .bucket(&bucket)
            .upload_id(&upload_id)
            .body(stream)
            .part_number(part_number)
            .send();

        let func = |part_number, this_chunk, total_size: String| async move {
            let res = upload_part_res.await;
            CompletedPart::builder()
                .e_tag(res.unwrap().e_tag.unwrap_or_default())
                .part_number(part_number)
                .build();

            upload_size += this_chunk;
            print!(
                "\rCpmpleted {}/{}",
                &get_size_in_nice(upload_size),
                total_size
            );
            stdout().flush().ok();
        };
        futures.push(func(part_number, this_chunk, total_size.clone()));
    }

    join_all(futures).await;

    //对upload_parts排序
    upload_parts.sort_by_key(|key| key.part_number());
    // 生成CompletedMultipartUpload对象
    let completed_multipart_upload: CompletedMultipartUpload = CompletedMultipartUpload::builder()
        .set_parts(Some(upload_parts))
        .build();
    // 合片
    let _complete_multipart_upload_res = client
        .complete_multipart_upload()
        .bucket(&bucket)
        .key(&key)
        .multipart_upload(completed_multipart_upload)
        .upload_id(upload_id)
        .send()
        .await?;
    println!("upload {:#} to s3://{:#}/{:#}", src, bucket, key);
    Ok(())
}

async fn list_object(client: &Client, target: &str) -> Result<(), Box<dyn Error>> {
    let (bucket, prefix) = parse_s3_url(target);
    let list_obj = client
        .list_objects_v2()
        .bucket(&bucket)
        .prefix(&prefix)
        .send()
        .await?;
    let contents = list_obj.contents().unwrap();
    for content in contents {
        println!(
            "{:<30}{:>10}  {:<1}",
            content
                .last_modified()
                .unwrap()
                .fmt(Format::DateTime)
                .unwrap(),
            get_size_in_nice(content.size() as u64),
            content.key().unwrap().replace(&prefix, ""),
        );
    }
    Ok(())
}

pub async fn sync_dir(
    client: &Client,
    src_dir: &str,
    target_dir: &str,
) -> Result<(), Box<dyn Error>> {
    for entry in WalkDir::new(src_dir) {
        match entry {
            Ok(src) => {
                if src.path().is_dir() {
                    continue;
                }
                let key = src.path().strip_prefix(src_dir)?;
                let target_key = Path::new(target_dir).join(key);
                let src = src.path().as_os_str().to_str().unwrap();
                let target_key = target_key.as_os_str().to_str().unwrap();
                let (bucket, key) = parse_s3_url(target_key);
                let get_object = client.get_object().bucket(&bucket).key(&key).send().await;
                match get_object {
                    Ok(_obj) => {}
                    Err(_e) => {
                        upload_file(client, src, target_key).await?;
                    }
                }
            }
            Err(_) => {}
        };
    }
    Ok(())
}

#[tokio::test]
async fn test_sync_dir() -> Result<(), Box<dyn Error>> {
    let client = &create_client();
    sync_dir(
        client,
        "/home/chengxuguang/code/rust/oss-client-rs/target/release",
        "s3://chengxuguang-bucket/s3-client-test/sync-test",
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_get_obj() -> Result<(), Box<dyn Error>> {
    let client = &create_client();
    sync_dir(
        client,
        "/home/chengxuguang/code/rust/oss-client-rs",
        "s3://chengxuguang-bucket/s3-client-test/sync-test/",
    )
    .await?;
    Ok(())
}

fn path_deal(src: &str, target: &str) -> (String, String) {
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
    (bucket, key)
}

fn parse_s3_url(url: &str) -> (String, String) {
    if !url.starts_with("s3:") {
        panic!("s3路径无效");
    }
    let mut components = Path::new(url).components();
    components.next();
    let bucket = components.next().unwrap().as_os_str().to_str().unwrap();
    let prefix_vec: Vec<_> = url.split(bucket).collect();
    let prefix = prefix_vec[1].trim_start_matches("/");

    (bucket.to_string(), prefix.to_string())
}

fn get_size_in_nice(size: u64) -> String {
    if size > GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else {
        format!("{:.2} MB", size as f64 / MB as f64)
    }
}

#[test]
fn parse_s3_url_test() {
    let (bucket, key) = parse_s3_url("s3://bucket/prefix1/prefix2/aaa.txt");
    assert_eq!(bucket, "bucket".to_string());
    assert_eq!(key, "prefix1/prefix2/aaa.txt".to_string());
}

#[tokio::test]
async fn test_mult_upload() -> Result<(), Box<dyn Error>> {
    let client = create_client();
    mutl_upload_v2(
        &client,
        "/home/chengxuguang/code/rust/oss-client-rs/target/release/s3-client-rs",
        "s3://chengxuguang-bucket/s3-client-test/",
    )
    .await?;
    Ok(())
}

#[test]
fn test_print_line() {
    for i in 0..10 {
        print!("\r{:?}", i);
        stdout().flush().ok();
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}

#[tokio::test]
async fn test_list_obj() -> Result<(), Box<dyn Error>> {
    let client = create_client();
    list_object(&client, "s3://chengxuguang-bucket/s3-client-test/").await?;
    Ok(())
}
