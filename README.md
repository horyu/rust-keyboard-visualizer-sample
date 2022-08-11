# keyboard-visualizer-sample

- Uses the swap chain model
- Splits render loop and message loop into separate threads
- Handles keyboard hook and message loop in the same thread
- Supports Windows OS

## Testing environment

```powershell
PS C:\Users\owner> [System.Environment]::OSVersion.Version

Major  Minor  Build  Revision
-----  -----  -----  --------
10     0      19044  0


PS C:\Users\owner> rustc -V
rustc 1.64.0-nightly (263edd43c 2022-07-17)
```

## License

See [LICENSE](./LICENSE).

This product uses [Microsoft](https://github.com/microsoft)/[windows-rs](https://github.com/microsoft/windows-rs).
