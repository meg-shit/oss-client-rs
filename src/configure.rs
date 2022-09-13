use std::env;
use std::fs;
use std::io::stdin;
use std::io::BufRead;
use std::io::BufReader;
use std::path::PathBuf;

#[derive(Debug)]
pub struct ConfigureValue {
    region: String,
    endpoint: String,
    access_key: String,
    secret_key: String,
}

impl ConfigureValue {
    pub fn new() -> ConfigureValue {
        let none = "None".to_string();
        return ConfigureValue {
            region: none.clone(),
            endpoint: none.clone(),
            access_key: none.clone(),
            secret_key: none.clone(),
        };
    }

    pub fn from_file(path: PathBuf) -> Self {
        let mut value = Self::new();
        match fs::File::open(path) {
            Ok(f) => {
                let fin = BufReader::new(f);
                for (idx, line) in fin.lines().enumerate() {
                    if idx == 0 {
                        continue;
                    }
                    let l = line.unwrap();
                    if l.starts_with("aws_access_key_id") {
                        value.access_key = value.parse_line(l);
                    } else if l.starts_with("aws_secret_access_key") {
                        value.secret_key = value.parse_line(l);
                    } else if l.starts_with("region") {
                        value.region = value.parse_line(l);
                    } else if l.starts_with("endpoint") {
                        value.endpoint = value.parse_line(l);
                    }
                }
            }
            Err(_) => {}
        }
        value
    }

    fn to_file(&self, path: PathBuf) {
        let parent = path.parent().unwrap();
        if !parent.exists() {
            let _ = fs::DirBuilder::new().create(path.parent().unwrap());
        };
        self._write(path);
    }

    fn _write(&self, path: PathBuf) {
        let contents = format!(
            "[default]\naws_access_key_id = {}\naws_secret_access_key = {}\nregion = {}\nendpoint = {}",
            self.access_key, self.secret_key, self.region, self.endpoint
        );
        fs::write(path, contents).unwrap();
    }

    fn parse_line(&self, line: String) -> String {
        let s: Vec<&str> = line.split("= ").collect();
        s[1].to_string()
    }
}

pub fn configure() {
    let config_filename = env::home_dir().unwrap().join(".aws/credentials");
    let mut value = ConfigureValue::from_file(config_filename.clone());
    let mut temp_str = String::new();

    println!("AWS Access Key ID [{:?}]:", value.access_key);
    stdin().read_line(&mut temp_str).unwrap();
    if temp_str.len() > 5 {
        value.access_key = temp_str.trim().to_string();
    }

    temp_str.clear();
    println!("AWS Secret Access Key [{:?}]:", value.secret_key);
    stdin().read_line(&mut temp_str).unwrap();
    if temp_str.len() > 5 {
        value.secret_key = temp_str.trim().to_string();
    }

    temp_str.clear();
    println!("AWS Region [{:?}]:", value.region);
    stdin().read_line(&mut temp_str).unwrap();
    if temp_str.len() > 2 {
        value.region = temp_str.trim().to_string();
    }

    temp_str.clear();
    println!("AWS Endpoint [{:?}]:", value.endpoint);
    stdin().read_line(&mut temp_str).unwrap();
    if temp_str.len() > 5 {
        value.endpoint = temp_str.trim().to_string();
    }

    value.to_file(config_filename);
}
