# WooDns 🪵🔧

![Status](https://img.shields.io/badge/status-in--development-orange)
![Language](https://img.shields.io/badge/Rust-stable-blue)
![License](https://img.shields.io/badge/license-MIT-green)

> 🚀 WooDns is a lightweight, fast local DNS server written in Rust for Docker environments. It automatically maps Docker container names to their internal IPs, enabling seamless service discovery and container networking—without manual configuration, host networking, or exposing ports just to access container services.


---

## 🚀 Why WooDns?

- 🎯 **Fast & Lightweight** – Built with async Rust, optimized for performance.
- 🐳 **Docker-Aware DNS Resolution** – Automatically resolves container names (`*.docker`) to their internal IPs, so you no longer need to use host networking or expose ports just to access container services.
- 🔁 **Live Updates** – Dynamically tracks Docker events to add or remove DNS records in real time when containers start or stop.
- 🌐 **Full DNS Compatibility** – Forwards all non-Docker queries to your preferred upstream DNS (e.g., Google DNS,Custom DNS).
- 🔄 **Zero Configuration** – Easily integrate with `/etc/resolv.conf` or `systemd-resolved`.
- 📦 **Single Binary** – Only one Rust executable, no extra dependencies.
- 👐 **Open Source** – MIT license, easy to contribute and audit.

## Who Should Use WooDns?

WooDns is ideal for:

- 🛠️ **Effortless Container Networking for Development**  
  Seamlessly access your containers by name, simplifying multi-container projects.
- 🧩 **Microservices Testing**  
  Quickly resolve service names to internal IPs for reliable integration and testing.
- 🚀 **DevOps Teams Needing Dynamic DNS for Containers**  
  Automatic DNS updates as containers start and stop, reducing manual network setup.
- 🏠 **Self-Hosted Environments**  
  Manage container networking in private labs or personal servers without external DNS.
- 🧑‍💻 **CI/CD Pipeline Environments**  
  Ensure repeatable, isolated network setups for automated testing and deployment.
- 🔄 **Legacy Systems Modernization**  
  Bridge old apps with new containerized services using DNS translation.

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

1. **Move the executable** to `/usr/local/bin/`:

    ```sh
    sudo mv target/release/WooDns-linux-amd64 /usr/local/bin/
    sudo chmod +x /usr/local/bin/WooDns-linux-amd64
    ```

2. **Create a systemd service file:**

    ```ini
    # /etc/systemd/system/woodns.service
    [Unit]
    Description=WooDns
    After=network.target docker.service

    [Service]
    ExecStart=/usr/local/bin/WooDns-linux-amd64
    # Change this to the user you want to run WooDns as root
    User=woodns
    Group=woodns
    Restart=on-failure
    AmbientCapabilities=CAP_NET_BIND_SERVICE
    Environment="fallback=127.0.x.x"
    # To run on different address(optional), default address is 127.0.0.13
    # Environment="host=127.0.x.x"
    #To change the log path (optional), default address is '/var/log/'
    #Environment="woodns_log_path=/mypath"
    [Install]
    WantedBy=multi-user.target
    ```

    To ensure proper functionality, your software requires setting an environment variable named fallback with the IP address of a DNS server. We recommend using a local DNS server for optimal performance.
    
    ### Recommended Approach
    - Extract the IP address of a nameserver from your system's DNS configuration file /etc/resolv.conf. You can use the first nameserver listed or any other nameserver IP address that suits your needs.
    - Set the fallback environment variable to the extracted IP address.

3. **Permissions:**

    - The `User` specified (here, `woodns`) **must**:
      - Have execute permissions on `/usr/local/bin/WooDns-linux-amd64`
      - Have Write permissions on `/var/log/` or the path in mentioned in 'woodns_log_path'
      - Have permissions to run Docker commands (typically by being in the `docker` group):
        ```sh
        sudo usermod -aG docker woodns
        ```
    
4. **Enable and start:**

    ```sh
    sudo systemctl daemon-reload
    sudo systemctl start woodns
    sudo systemctl enable woodns
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
