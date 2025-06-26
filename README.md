# BPMNCode

A textual DSL (Domain Specific Language) for describing BPMN 2.0 processes that automatically generates valid BPMN 2.0 diagrams.

## Installation

```bash
# Clone the repository
git clone git@github.com:BPMNCode/BPMNCode.git
cd BPMNCode

# Build the project
cargo build --release

# Install globally (optional)
cargo install --path .
```

## Usage

### Basic Commands

```bash
# Validate syntax
bpmncode check examples/simple.bpmn

# Show syntax information
bpmncode info
```

## Syntax Overview

### Basic Process Structure

```bpmn
process OrderFlow @version "1.0" {
    start
    task ValidateOrder
    xor InStock? {
        yes -> ShipOrder
        no -> NotifyCustomer
    }
    ShipOrder -> end
    NotifyCustomer -> end
}
```

### Supported Elements

| Element        | Syntax                                                  | Description                             |
| -------------- | ------------------------------------------------------- | --------------------------------------- |
| **Process**    | `process Name { ... }`                                  | Root container                          |
| **Events**     | `start`, `end`                                          | Start/End events                        |
| **Tasks**      | `task Name`, `user Name`, `service Name`, `script Name` | Different task types                    |
| **Gateways**   | `xor Name? { ... }`, `and Name`                         | Decision and parallel gateways          |
| **Flows**      | `->`, `-->`, `=>`, `..>`                                | Sequence, message, default, association |
| **Containers** | `pool Name { ... }`, `lane Name { ... }`                | Process participants                    |
| **Subprocess** | `subprocess Name { ... }`                               | Embedded processes                      |
| **Groups**     | `group "Name" { ... }`                                  | Visual grouping                         |

### Advanced Features

#### Imports and Modularity

```
// Import entire file with namespace
import "flows/payment.bpmn" as payment
import "common/validators.bpmn" as validators

// Import specific elements
import subprocess PaymentFlow, DataValidation from "flows/common.bpmn"

process MainFlow {
    start
    call payment::ProcessPayment
    call validators::ValidateData
    call PaymentFlow  // Direct import
    end
}
```

#### Task Attributes

```
task ValidateOrder (async=true retries=3 timeout=30s)
user SignContract (assignee="manager" priority=high)
service CallAPI (endpoint="https://api.example.com")
```

#### Annotations

```
process OrderFlow @version "1.2" @author "Team Lead" @sla "2h" {
    start @message "OrderReceived"
    task ProcessOrder
    end @error "ProcessingFailed"
}
```

#### Pools and Lanes

```
pool CustomerService {
    lane FrontOffice {
        task ReceiveOrder
        task ValidateOrder
    }
    lane BackOffice {
        task ProcessPayment
        task ShipOrder
    }
}

pool Warehouse {
    task PackOrder
    task UpdateInventory
}

// Cross-pool communication
CustomerService.ProcessPayment --> Warehouse.PackOrder
```

#### Conditional Flows

```
xor PaymentValid? {
    [amount > 0 && currency == "USD"] -> ProcessPayment
    [amount <= 0] -> RejectOrder
    => HandleSpecialCase  // default flow
}
```

#### Comments

```
// Single line comment
process OrderFlow {
    /* Multi-line comment
       describing complex logic */
    start
    task ValidateOrder  // Inline comment
    end
}
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run lexer tests specifically
cargo test lexer

# Run with output
cargo test -- --nocapture

# Test CLI commands
cargo run check examples/simple.bpmn
cargo run info
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/feature-name`)
3. Commit your changes (`git commit -m 'Add new feature'`)
4. Push to the branch (`git push origin feature/feature-name`)
5. Open a Pull Request

### Development Setup

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and setup
git clone https://github.com/BPMNCode/BPMNCode
cd BPMNCode
cargo build

# Run tests
cargo test

# Install development tools
cargo install cargo-watch
cargo watch -x test  # Auto-run tests on changes
```
