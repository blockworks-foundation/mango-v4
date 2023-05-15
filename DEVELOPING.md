_work in progress_

### Tooling

- Consider enabling the `enable-gpl` and `test-bpf` features in your rust ide to get sensible tooling for all parts of the source code.

![image](https://user-images.githubusercontent.com/89031858/219597204-4c51d1ee-26d2-4118-8688-c72ac421022a.png)


### Code style

### Testing

In order to run the tests the `enable_gpl` feature needs to be enabled to not skip essential tests.

```
cargo test-sbf --features enable-gpl
```