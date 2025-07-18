# WooDns 🪵🔧

![Status](https://img.shields.io/badge/status-in--development-orange)
![Language](https://img.shields.io/badge/Rust-stable-blue)
![License](https://img.shields.io/badge/license-MIT-green)

> 🚀 A lightweight, blazing-fast local DNS server written in Rust. Maps Docker container names to their internal IPs, and seamlessly forwards all other DNS queries to your upstream resolver.

---

## ✨ Features

- 🔧 **Resolve Docker container names** to their internal IPs instantly
- 🌍 **Automatic forwarding** of all other DNS queries to your upstream DNS (like 8.8.8.8)
- 🐳 No need to expose container ports to the host
- ⚡ Built with Rust async for performance and reliability
- 🛠️ Easy integration via `/etc/resolv.conf` or `systemd-resolved`
- 📦 Minimal dependencies, single binary

---

## ⚠️ Project Status: In Development

This project is still under active development.  
**Bugs, missing features, or small issues may be present.**

> Contributions, bug reports, and suggestions are highly welcome!

---



## 🚀 Quick Start

### 1. Download or Build

- **Option A: Download Compiled Binary (Linux x86_64)**
  
  [Download Latest Release](./WooDns-linux-amd64)

  ```sh
  chmod +x WooDns-linux-amd64
  ```

- **Option B: Build from Source**

  ```sh
  git clone https://github.com/Subhodip1307/WooDns.git
  cd WooDns
  cargo build --release
  # The binary will be at target/release/woodns
  ```

---

### 2. Deploy with systemd

1. **Move the executable** to `/usr/local/bin/woodns`:

    ```sh
    sudo mv target/release/woodns /usr/local/bin/woodns
    sudo chmod +x /usr/local/bin/woodns
    ```

2. **Create a systemd service file:**

    ```ini
    # /etc/systemd/system/woodns.service
    [Unit]
    Description=WooDns Docker-aware local DNS server
    After=network.target docker.service

    [Service]
    ExecStart=/usr/local/bin/woodns
    # Change this to the user you want to run WooDns as
    User=woodns
    Group=woodns
    Restart=on-failure
    AmbientCapabilities=CAP_NET_BIND_SERVICE
    # To run on different address(optional), default address is 127.0.0.13
    # Environment="host=127.0.x.x"
    [Install]
    WantedBy=multi-user.target
    ```

3. **Permissions:**

    - The `User` specified (here, `woodns`) **must**:
      - Have execute permissions on `/usr/local/bin/woodns`
      - Have permissions to run Docker commands (typically by being in the `docker` group):
        ```sh
        sudo usermod -aG docker woodns
        ```
      - Own or have read access to any config files if you add them

4. **Enable and start:**

    ```sh
    sudo systemctl daemon-reload
    sudo systemctl enable woodns
    sudo systemctl start woodns
    sudo systemctl status woodns
    ```

---

### 3. Configure systemd-resolved to Use WooDns

1. **Edit resolved.conf:**
    ```sh
    sudo nano /etc/systemd/resolved.conf
    ```
    Add or modify the following in the `[Resolve]` section:
    ```
    [Resolve]
    DNS=127.0.0.13
    ```
    Replace `127.0.0.13` with the IP address where WooDns is running if different.

2. **Restart systemd-resolved:**
    ```sh
    sudo systemctl restart systemd-resolved
    ```

---

## 🐳 Demo: Resolving Docker Container Names

Once WooDns is running and your system points to it for DNS:

```sh
# List all running containers
docker container ls

# Example output:
# CONTAINER ID   IMAGE        COMMAND                  NAMES
# a1b2c3d4e5f6   nginx:alpine "nginx -g 'daemon of…"   nginx_demo
# 2345f6a7b8c9   redis:alpine "docker-entrypoint.s…"   redis_cache

# To ping a container, use the format:
ping nginx_demo.docker

# You should see replies from the container's internal IP!
```

---

## 🔄 Use Container Names in Host Applications

Thanks to WooDns, you can now use Docker container names (with the `.docker` suffix) **anywhere on your host machine** where a hostname is accepted, such as:

- **Nginx reverse proxy configs**:
    ```nginx
    upstream backend {
        server redis_cache.docker:6379;  # Use the container name with .docker!
    }
    ```
- **Other application configs** (e.g., databases, microservices, etc.):
    ```
    host = nginx_demo.docker
    ```

No more looking up or hardcoding container IPs—just use the container name with `.docker`!

---


## 🛠️ How It Works

- Resolves Docker container names (with `.docker` suffix) to their IPs automatically.
- If a queried name is not a Docker container, WooDns forwards the request to your upstream DNS (e.g., 8.8.8.8).
- No need for port exposure or manual `/etc/hosts` editing.

---

## ⚠️ Notes & Security

- **User permissions:** The systemd service must run as a user with permission to execute the WooDns binary and access Docker (usually by being in the `docker` group).
- **Port:** WooDns binds to `127.0.0.13:53` by default. Ensure nothing else is using this address/port.
- **Production:** WooDns is under active development. Test thoroughly before deploying in critical environments.

---

## 🤝 Contributing

Bugs, feature requests, and PRs are very welcome!  
Please open an issue or submit a pull request.

---

## 📄 License

MIT License.  
See [LICENSE](./LICENSE) for details.

---
