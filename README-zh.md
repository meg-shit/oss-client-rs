# s3-client-rs

> 用rust编写的一款s3文件操作的cli程序

## 功能

- s3 configure               凭证配置

  ```bash
  AWS Access Key ID [****************eEZJ]: 
  AWS Secret Access Key [****************nIhy]: 
  Default region name [None]: 
  Default endpoint link [None]:
  ```

- s3 cp                       上传并且覆盖单个文件
- s3 sync                     上传但不覆盖文件夹

## TODO

- [x] cli配置
- [ ] 判断文件或路径是local还是s3
- [x] 分片上传
- [x] 断点续传
- [x] 进度显示
- [ ] 当前网速显示
- [x] 实现sync功能，已上传文件不重复上传
- [ ] s3 mock支持
