## install

### through the [installer script](https://raw.githubusercontent.com/vahrina/sheikah/main/install.sh)

```sh
curl -fsSL https://raw.githubusercontent.com/vahrina/sheikah/main/install.sh -o install.sh \
  && less $_
bash install.sh
```

### manual installation

grab the desired `sheikah-vX.Y.Z-<arch>.tar.gz` for your cpu architechture from [releases](https://github.com/vahrina/sheikah/releases/latest), then:

```sh
tar -xzf sheikah-v0.2.0-x86_64.tar.gz
install -m 755 sheikah ~/.local/bin/
```

### compiling from source

needs a rust toolchain & preferably [`just`](https://github.com/casey/just#installation)

```sh
git clone https://github.com/vahrina/sheikah && cd sheikah
just install # or: cargo install --path . --locked
```
