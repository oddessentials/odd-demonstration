# 📡 Distributed Task Observatory - Complete Step-by-Step Guide

Welcome! This guide will walk you through setting up and running the Distributed Task Observatory from scratch. Follow each step in order.

> **📌 Platform Note:** This guide is written for **Windows** users. The Distributed Task Observatory supports **all platforms** (Windows, macOS, Linux). For macOS/Linux users:
> - Install Docker Desktop for your platform
> - Install PowerShell Core: `brew install powershell` (macOS) or [Linux instructions](https://aka.ms/install-powershell)
> - Install kind and kubectl via Homebrew or your package manager
> - Run scripts with: `pwsh ./scripts/start-all.ps1`

---

## Part 1: Installing Required Software

### Step 1: Open PowerShell as Administrator
1. Press the **Windows key** on your keyboard
2. Type `PowerShell`
3. Right-click on **Windows PowerShell**
4. Click **Run as administrator**
5. Click **Yes** when prompted

### Step 2: Install the Chocolatey Package Manager
Copy and paste this entire command into PowerShell, then press Enter:
```powershell
Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))
```
Wait for it to complete. You should see "Chocolatey is ready."

### Step 3: Install Docker Desktop
```powershell
choco install docker-desktop -y
```
**Important:** After this completes, restart your computer.

### Step 4: Start Docker Desktop
1. After restarting, find **Docker Desktop** in your Start menu
2. Open it and wait for it to finish starting
3. Look for a whale icon in your system tray (bottom-right of screen)
4. The whale should be stable (not animating) when Docker is ready

### Step 5: Install kubectl
Open PowerShell as Administrator again and run:
```powershell
choco install kubernetes-cli -y
```

### Step 6: Install kind (Kubernetes in Docker)
```powershell
choco install kind -y
```

### Step 7: Install Git
```powershell
choco install git -y
```

### Step 8: Install Rust (for the Terminal UI)
```powershell
choco install rustup.install -y
```

### Step 9: Close and Reopen PowerShell
Close PowerShell completely and open a new PowerShell window (doesn't need to be Administrator anymore).

### Step 10: Verify Everything is Installed
Run these commands one at a time:
```powershell
docker --version
kubectl version --client
kind --version
git --version
rustc --version
```
Each should show a version number. If any show an error, go back and redo that installation step.

---

## Part 2: Getting the Project

### Step 11: Choose Where to Put the Project
Navigate to a folder where you want the project. For example:
```powershell
cd C:\Users\YourName\Documents
```
(Replace `YourName` with your actual Windows username)

### Step 12: Clone the Project
```powershell
git clone https://github.com/YOUR_USERNAME/odd-demonstration.git
```
(Replace `YOUR_USERNAME` with the actual GitHub username)

### Step 13: Enter the Project Folder
```powershell
cd odd-demonstration
```

---

## Part 3: Creating the Kubernetes Cluster

### Step 14: Create the Cluster
```powershell
.\scripts\setup-cluster.ps1
```
This takes 1-3 minutes. Wait until you see "Cluster is ready!"

### Step 15: Verify the Cluster is Running
```powershell
kubectl get nodes
```
You should see a line containing `task-observatory-control-plane` with `Ready` status.

---

## Part 4: Building the Application

### Step 16: Build the Gateway Service
```powershell
docker build -t gateway:latest -f src/services/gateway/Dockerfile .
```
Wait for "Successfully tagged gateway:latest"

### Step 17: Build the Processor Service
```powershell
docker build -t processor:latest -f src/services/processor/Dockerfile .
```
Wait for "Successfully tagged processor:latest"

### Step 18: Build the Metrics Engine
```powershell
docker build -t metrics-engine:latest -f src/services/metrics-engine/Dockerfile src/services/metrics-engine
```

### Step 19: Build the Read Model
```powershell
docker build -t read-model:latest -f src/services/read-model/Dockerfile src/services/read-model
```

### Step 20: Build the Web Dashboard
```powershell
docker build -t web-ui:latest -f src/interfaces/web/Dockerfile src/interfaces/web
```

---

## Part 5: Loading Images into Kubernetes

### Step 21: Load All Images
Run each command one at a time:
```powershell
kind load docker-image gateway:latest --name task-observatory
kind load docker-image processor:latest --name task-observatory
kind load docker-image metrics-engine:latest --name task-observatory
kind load docker-image read-model:latest --name task-observatory
kind load docker-image web-ui:latest --name task-observatory
```

---

## Part 6: Deploying to Kubernetes

### Step 22: Deploy Everything
```powershell
kubectl apply -f .\infra\k8s\
```

### Step 23: Wait for All Services to Start
```powershell
kubectl get pods --watch
```
Wait until ALL pods show `1/1` under READY and `Running` under STATUS.
Press **Ctrl+C** to stop watching once everything is running.

This typically takes 2-3 minutes.

---

## Part 7: Connecting to the Services

You need to run the following commands, each in a **separate PowerShell window**. Keep all windows open while using the system.

### Step 24: Open Six PowerShell Windows
Open 6 new PowerShell windows. In each one, navigate to the project:
```powershell
cd C:\Users\YourName\Documents\odd-demonstration
```

### Step 25: In Window 1 - Connect Gateway
```powershell
kubectl port-forward svc/gateway 3000:3000
```
Leave this running.

### Step 26: In Window 2 - Connect Read Model API
```powershell
kubectl port-forward svc/read-model 8080:8080
```
Leave this running.

### Step 27: In Window 3 - Connect Web Dashboard
```powershell
kubectl port-forward svc/web-ui 8081:80
```
Leave this running.

### Step 28: In Window 4 - Connect RabbitMQ
```powershell
kubectl port-forward svc/rabbitmq 15672:15672
```
Leave this running.

### Step 29: In Window 5 - Connect Grafana
```powershell
kubectl port-forward svc/grafana 3002:3000
```
Leave this running.

### Step 30: In Window 6 - Connect Prometheus
```powershell
kubectl port-forward svc/prometheus 9090:9090
```
Leave this running.

---

## Part 8: Opening the User Interfaces

### Step 31: Open the Web Dashboard
Open your web browser and go to:
```
http://localhost:8081
```
You should see a dark themed dashboard with "Distributed Task Observatory" at the top.

### Step 32: Open RabbitMQ Management
Go to:
```
http://localhost:15672
```
Login with:
- **Username:** guest
- **Password:** guest

### Step 33: Open Grafana
Go to:
```
http://localhost:3002
```
Login with:
- **Username:** admin
- **Password:** admin

If asked to change password, click "Skip" or set a new one.

### Step 34: Open the Grafana Dashboard
1. Click the hamburger menu (☰) on the left
2. Click **Dashboards**
3. Click **Distributed Task Observatory**

### Step 35: Open Prometheus
Go to:
```
http://localhost:9090
```
No login required.

---

## Part 9: Running the Terminal UI (TUI)

### Step 36: Open a New PowerShell Window
Navigate to the TUI folder:
```powershell
cd C:\Users\YourName\Documents\odd-demonstration\src\interfaces\tui
```

### Step 37: Build the TUI
```powershell
cargo build --release
```
This takes 2-5 minutes the first time.

### Step 38: Run the TUI
```powershell
$env:READ_MODEL_URL="http://localhost:8080"; cargo run --release
```
You should see a terminal-based dashboard with job statistics.

**TUI Controls:**
- Press `q` to quit
- Press `r` to refresh

---

## Part 10: Submitting Test Jobs

Now let's see the system in action by submitting three test jobs!

### Step 39: Open a New PowerShell Window
Navigate to the project:
```powershell
cd C:\Users\YourName\Documents\odd-demonstration
```

### Step 40: Submit Job 1
Copy and paste this entire block:
```powershell
$job1 = @{
    id = [guid]::NewGuid().ToString()
    type = "data-processing"
    status = "PENDING"
    createdAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    payload = @{ task = "Process customer data"; priority = "high" }
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://localhost:3000/jobs" -Method Post -Body $job1 -ContentType "application/json"
```
You should see a response with `jobId` and `status: accepted`.

### Step 41: Submit Job 2
```powershell
$job2 = @{
    id = [guid]::NewGuid().ToString()
    type = "report-generation"
    status = "PENDING"
    createdAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    payload = @{ report = "Monthly sales summary"; format = "PDF" }
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://localhost:3000/jobs" -Method Post -Body $job2 -ContentType "application/json"
```

### Step 42: Submit Job 3
```powershell
$job3 = @{
    id = [guid]::NewGuid().ToString()
    type = "notification"
    status = "PENDING"
    createdAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    payload = @{ recipient = "team@example.com"; message = "Weekly update" }
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://localhost:3000/jobs" -Method Post -Body $job3 -ContentType "application/json"
```

---

## Part 11: Watching the Jobs Flow Through the System

### Step 43: Check the Web Dashboard
Go to http://localhost:8081
- The counters should now show 3 jobs
- The table should show all three jobs with status COMPLETED

### Step 44: Check Grafana
Go to http://localhost:3002 → Dashboards → Distributed Task Observatory
- The "Jobs Submitted" panel should show 3
- The "Jobs Completed" panel should show 3

### Step 45: Check the TUI
If the TUI is still running, press `r` to refresh.
You should see the updated statistics.

### Step 46: Check RabbitMQ
Go to http://localhost:15672 → Queues
You can see the message flow through the queues.

---

## Part 12: Shutting Down

### Step 47: Stop All Port Forwards
Go to each PowerShell window running a port-forward and press **Ctrl+C**.

### Step 48: Delete the Kubernetes Cluster
In any PowerShell window:
```powershell
kind delete cluster --name task-observatory
```

---

## Troubleshooting

### "The term 'xyz' is not recognized"
The software wasn't installed correctly. Close PowerShell, reopen it, and try the command again. If it still doesn't work, reinstall that software.

### Port forward says "error: lost connection to pod"
The service restarted. Just run the port-forward command again.

### Docker Desktop won't start
Make sure virtualization is enabled in your BIOS. Search online for "enable virtualization [your computer model]".

### Web page shows "connection refused"
Make sure the corresponding port-forward is running in a PowerShell window.

### Pods stuck in "Pending" or "ContainerCreating"
Wait a few more minutes. If still stuck after 5 minutes, run:
```powershell
kubectl describe pod NAME_OF_POD
```
(Replace NAME_OF_POD with the actual pod name from `kubectl get pods`)

---

## Summary of URLs

| Service | URL | Login |
|---------|-----|-------|
| Web Dashboard | http://localhost:8081 | None |
| RabbitMQ | http://localhost:15672 | guest / guest |
| Grafana | http://localhost:3002 | admin / admin |
| Prometheus | http://localhost:9090 | None |
| Gateway API | http://localhost:3000 | None |
| Read Model API | http://localhost:8080/stats | None |

---

Congratulations! You've successfully set up and run the Distributed Task Observatory! 🎉
