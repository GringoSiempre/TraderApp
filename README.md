# TraderApp by Moonward Labs

**TraderApp** is a tool for speculative trading of options on major indices through ETF QQQ and SPY on **Freedom24** brokerage accounts. The application automates trading processes, making them faster and more efficient.

---
## 🚀 Key Features

- **Automatic selection of option tickers** for entering a position.
- **One-click position opening** for quick market execution.
- **Real-time monitoring of current positions** with key metrics displayed.
- **Smart stop-loss system**:
  - Automatically limits losses with minimal delay.
- **Real-time quotes** for effective market analysis.
- **Secure data storage** using **AES-256 encryption** for credentials.

---
## ⚠️ Requirements

The application requires **predefined configuration files** containing account details and credentials for connecting to the **Freedom24** brokerage.  

> **Without these files, the program will not work properly.**

---
## 🛠 Technologies

TraderApp is developed using **Rust** and the following libraries:

- **egui** — for creating the graphical user interface.
- **serde_json** — for JSON data handling.
- **tokio** — for asynchronous task execution.
- **WebSocket** — for real-time quotes.
- **POST requests** — for order execution via the broker's API.
- **AES-256** — for encrypted and secure storage of credentials.

---
## ⚙️ Installation and Setup

### 1. **Clone the Repository**
`git clone https://github.com/your-username/TraderApp.git cd TraderApp`

### 2. **Prepare Configuration Files**
Before running the application, ensure you have the required configuration files with encrypted credentials.
### 3. **Build the Project**
Make sure **Rust** and **Cargo** are installed:
`cargo build --release`

### 4. **Run the Application**
`cargo run`

---
## 🔒 Data Security

- Credentials are securely encrypted using **AES-256**.

---
## 📊 How It Works

1. **Ticker Selection**: The app automatically identifies the most suitable option tickers for QQQ and SPY.
2. **Position Execution**: A single button click sends orders to the broker using **POST requests**.
3. **Smart Stop-Loss**: The system manages stop-loss triggers to minimize losses effectively.
4. **Position Monitoring**: Current positions and real-time quotes are displayed in the interface.

---
## 🖥 Screenshots

_Will be added later_

---
## 📄 License

This project is licensed under the **MIT License**. See the LICENSE file for details.

---
## ✨ Contact

For any questions or suggestions, feel free to reach out:  
**ss.cz@icloud.com**

Alternatively, open an **Issue** in this repository.