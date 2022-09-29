# s3-client-rs

> A command line tool for s3 operation by rust

[中文文档](./README-zh.md)

## feature

- s3 configure       *credentials configure*

  ```bash
  AWS Access Key ID [****************eEZJ]: 
  AWS Secret Access Key [****************nIhy]: 
  Default region name [None]: 
  Default endpoint link [None]:
  ```

- s3 cp              *Upload and overwrite single file*

- s3 sync            *Upload directories but not overwrite*

## TODO

- [x] Cli configure
- [ ] Distinguish local path and s3 path
- [x] Multipart upload
- [x] breakpoint continue
- [x] progress display
- [ ] network status display
- [ ] sync feature: don't repeat upload
