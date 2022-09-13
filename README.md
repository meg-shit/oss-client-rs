# s3-client-rs

> 用rust编写的一款s3文件操作的cli程序

## 功能

- s3 configure                配置环境

  ```bash
  AWS Access Key ID [****************eEZJ]: 
  AWS Secret Access Key [****************nIhy]: 
  Default region name [None]: 
  Default endpoint link [None]:
  ```

- s3 cp
  - local to s3
  - s3 to s3
  - s3 to local
- s3 sync
  - local to s3
  - s3 to s3
  - s3 to local

## 功能点拆分

- cli配置
- 判断文件或路径是local还是s3
- 实现分片上传和断点续传
- 进度显示、当前网速显示
- 实现sync功能，已上传文件不重复上传