# rcat: a netcat clone

Documentation for the Linux `netcat` command is found [here](https://linux.die.net/man/1/nc).

## Testing the client connect against netcat

Using the Docker image provided at the root of the repository, build the project

```shell
cargo install --path rcat
rcat --version
```

Start a netcat listener using 

```shell
nc -l -p 2323
``` 

For the client you will need a second shell from your machine. Find the container that's already running using `docker ps` and look for a two part name that looks like `infallible_almeida`. Start a client shell using `docker exec -it infallible_almeida sh`.

Now you're back in with a second shell, you can run 

```shell
rcat connect localhost --port 2323
```

Switch back to the first shell to see the output.
