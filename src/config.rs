use configparser::ini::Ini;
use std::{
    collections::HashMap,
    env,
    error::Error,
    fs,
    io::{stdin, stdout, Write},
};

pub fn parser() -> Result<(), Box<dyn Error>> {
    // 确保配置文件存在
    let home = env::home_dir().expect("home目录获取失败");
    let config_dir = home.join(".s3");
    let config_file = config_dir.join("credentials");
    if !config_dir.exists() {
        fs::DirBuilder::new().create(config_dir)?;
    };
    if !config_file.exists() {
        fs::File::create(&config_file)?;
    };
    // 读取配置文件
    let mut config = Ini::new();
    let map = config.load(&config_file)?;

    let mut default: HashMap<String, Option<String>> = HashMap::new();
    match map.get("default") {
        Some(d) => {
            default = d.clone();
        }
        None => {}
    }

    let keys = [
        "aws_access_key_id",
        "aws_secret_access_key",
        "region",
        "endpoint",
    ];
    let names = [
        "AWS Access Key ID",
        "AWS Secret Access Key",
        "AWS Region",
        "AWS Endpoint",
    ];

    // 更新配置
    for idx in 0..keys.len() {
        if let Some(new_value) = overwrite(names[idx], default.get(keys[idx])) {
            config.set("default", keys[idx], Some(new_value));
        }
    }
    // 保存配置
    config.write(config_file)?;
    Ok(())
}

fn overwrite(name: &str, value: Option<&Option<String>>) -> Option<String> {
    let mut _v = String::new();
    match value {
        Some(v_opt) => match v_opt {
            Some(v) => {
                _v = v.to_string();
            }
            None => _v = "None".to_string(),
        },
        None => _v = "None".to_string(),
    }
    print!("{:?} [{:?}]:", name, _v);
    let _ = stdout().flush();
    let mut temp_str = String::new();
    stdin().read_line(&mut temp_str).unwrap();
    let new_value = temp_str.trim().to_string();
    if new_value != "" {
        Some(new_value)
    } else {
        None
    }
}
