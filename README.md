# netrusting

Networking with Rust

## Setting up for development

Build the dev image with `docker build -t netrusting .` then launch the image with

On Windows with Powershell
```powershell
docker run -it -v "$(pwd):/netrusting" -v cargo_cache:/usr/local/cargo/ netrusting
```

On macOS or Linux with Bash or Zsh
```powershell
docker run -it -v "`pwd`:/netrusting" -v cargo_cache:/usr/local/cargo/ netrusting
```
