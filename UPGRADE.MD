## 🛠️ Upgrade Instructions

### ✅ Option 1: Download Pre-built Binary (Recommended)

Download the latest Linux pre-built binary from GitHub Releases:

```sh
wget https://github.com/Subhodip1307/WooDns/releases/latest/download/WooDns-linux-amd64
chmod +x WooDns-linux-amd64
sudo mv WooDns-linux-amd64 /usr/local/bin/
```

### 🛠️ Option 2: Build from Source

Clone and build latest release:

````sh
git clone https://github.com/Subhodip1307/WooDns.git
cd WooDns
cargo build --release
````

2. Replace existing binary:

```sh
sudo mv target/release/woodns /usr/local/bin/woodns
```

3. Restart service (if using systemd):

```sh
sudo systemctl restart woodns
```
## 🛠️ Upgrade Instructions

### ✅ Option 1: Download Pre-built Binary (Recommended)

Download the latest Linux pre-built binary from GitHub Releases:


```sh
wget https://github.com/Subhodip1307/WooDns/releases/latest/download/WooDns-linux-amd64
chmod +x WooDns-linux-amd64
sudo mv WooDns-linux-amd64 /usr/local/bin/
```

### 🛠️ Option 2: Build from Source

1, Clone and build latest release:

````sh
git clone https://github.com/Subhodip1307/WooDns.git
cd WooDns
cargo build --release
````


2. Replace existing binary:

```sh
sudo mv target/release/woodns /usr/local/bin/woodns
```

3. Restart service (if using systemd):

```sh
 sudo systemctl restart woodns
```