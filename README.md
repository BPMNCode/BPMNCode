# BPMNCode

A modern textual DSL (Domain Specific Language) for describing BPMN 2.0 processes with advanced validation, error recovery, and comprehensive syntax support.

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
# Validate BPMN syntax
bpmncode check examples/simple.bpmn

# Validate with verbose output
bpmncode check --verbose examples/complex.bpmn

# Show syntax information
bpmncode info

# Check all examples
bpmncode check examples/*.bpmn
```

## Syntax Overview

### Basic Process Structure

```bpmn
process OrderFlow @version "1.0" @author "Developer" {
    start @message "OrderReceived"
    
    task ValidateOrder(timeout=300s, assignee="validator")
    user ReviewOrder(assignee="manager", priority=high)
    
    xor OrderValid? {
        [validation_result == "valid"] -> ProcessOrder
        [validation_result == "invalid"] -> RejectOrder
        => ManualReview  // default flow
    }
    
    task ProcessOrder
    task RejectOrder
    user ManualReview
    
    end @message "OrderCompleted"
    
    // Flows
    ValidateOrder -> OrderValid
    ProcessOrder -> end
    RejectOrder -> end
    ManualReview -> ProcessOrder
}
```

### Supported Elements

| Element        | Syntax                                                    | Description                             |
| -------------- | --------------------------------------------------------- | --------------------------------------- |
| **Process**    | `process Name @attr "value" { ... }`                     | Root container with metadata            |
| **Events**     | `start @type "trigger"`, `end @type "result"`            | Start/End events with types             |
| **Tasks**      | `task Name(attr=value)`, `user Name`, `service Name`     | Work items with attributes              |
| **Gateways**   | `xor Name? { [condition] -> target }`, `and Name { ... }` | Decision and parallel gateways          |
| **Flows**      | `->`, `-->`, `=>`, `..>`                                 | Sequence, message, default, association |
| **Containers** | `pool Name { lane Lane { ... } }`                        | Process participants with swimlanes     |
| **Subprocess** | `subprocess Name(attr=value) { ... }`                    | Embedded processes                      |
| **Groups**     | `group "Label" { ... }`                                  | Visual grouping of elements             |
| **Events**     | `event Name @type "trigger"`                             | Intermediate events                     |
| **Call**       | `call ProcessName(param=value)`                          | External process invocation             |
| **Annotations**| `note "Documentation text"`                              | Process documentation                   |

### Advanced Features

#### Imports and Modularity

```bpmn
// Import entire file with namespace
import "flows/payment.bpmn" as payment
import "common/validators.bpmn" as validators

// Import specific elements
import PaymentFlow, DataValidation from "flows/common.bpmn"

process MainFlow {
    start
    call PaymentFlow(simplified=true)
    call DataValidation(strict=false)
    end
}
```

#### Task Attributes

```bpmn
task ValidateOrder(timeout=300s, assignee="validator", priority=high)
user SignContract(assignee="manager", form="contract-form", required=true)
service CallAPI(endpoint="/api/pricing", method="POST", timeout=30s)
script UpdateDatabase(script="update_order.sql", params="order_id,status")
```

#### Event Types and Annotations

```bpmn
process OrderFlow @version "1.2" @author "Team Lead" @description "Order processing" {
    start @message "OrderReceived"
    start @timer "daily"
    
    event WaitForPayment @message "PaymentConfirmed"
    event ErrorHandler @error "ProcessingError"
    event SignalCatcher @signal "ManagerApproval"
    
    task ProcessOrder
    
    end @message "OrderCompleted"
    end @error "ValidationFailed"
    end @terminate "ProcessCancelled"
}
```

#### Pools and Lanes

```bpmn
pool Customer {
    lane OnlineCustomer {
        task PlaceOrder
        task ConfirmOrder
        user ProvidePayment(secure=true)
    }
    
    lane PremiumCustomer {
        task ExpressOrder(priority=urgent)
        service AutoApproval(instant=true)
    }
}

pool Supplier {
    task CheckAvailability
    service UpdateInventory(realtime=true)
    event NotifyDelay @timer "2h"
}

// Message flows between pools
PlaceOrder --> CheckAvailability
UpdateInventory --> ConfirmOrder
```

#### Gateway Conditions and Flows

```bpmn
// Exclusive (XOR) gateway with conditions
xor PaymentValid? {
    [amount > 0 && currency == "USD"] -> ProcessPayment
    [amount <= 0] -> RejectOrder
    [currency != "USD"] -> CurrencyConversion
    => HandleSpecialCase  // default flow
}

// Parallel (AND) gateway
and ParallelProcessing {
    [split] -> InventoryCheck
    [split] -> CreditCheck
    [split] -> ComplianceCheck
}

and ParallelJoin {
    [join] -> ProcessOrder
}
```

#### Subprocesses and Documentation

```bpmn
// Subprocess with nested elements
subprocess OrderFulfillment(collapsed=false) {
    start
    task PickItems(assignee="warehouse")
    task PackItems(parallel=true)
    task LabelPackage
    end @message "OrderFulfilled"
}

// Groups for organization
group "ValidationGroup" {
    task ValidateOrder
    task ReviewOrder
    user ManagerApproval(escalation=true)
}

// Annotations
note "This process handles complex order scenarios"
note "SLA: 4 hours, Success rate: 95%"

// Comments
// Single line comment
/* Multi-line comment
   describing complex logic */
```

## Examples

The `examples/` directory contains comprehensive BPMN process examples:

- `examples/simple.bpmn` - Basic process structure
- `examples/complex.bpmn` - Advanced features and flows  
- `examples/comprehensive.bpmn` - Complete demonstration of all supported elements

```bash
# Check all examples
cargo run check examples/*.bpmn

# View detailed AST structure
cargo run check --verbose examples/comprehensive.bpmn
```

## Testing

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test lexer
cargo test parser
cargo test validator

# Run with output
cargo test -- --nocapture

# Test CLI functionality
make examples  # Run all example files
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

# Code formatting and linting
cargo fmt --check  # Check formatting
cargo fmt          # Apply formatting  
cargo clippy -- -D warnings  # Lint with strict warnings

# Development tools
cargo install cargo-watch
cargo watch -x test    # Auto-run tests on changes
cargo watch -x check   # Auto-check on changes
```

## Features

- **Comprehensive BPMN 2.0 Support**: All major BPMN elements and flow types
- **Advanced Parsing**: Robust error recovery and detailed syntax validation
- **Rich Syntax**: Attributes, annotations, conditions, and nested structures
- **Modular Design**: Import system for process composition
- **Developer Friendly**: Clear error messages and verbose debugging output
- **Performance**: Fast lexing and parsing with optimized data structures
