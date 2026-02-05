# iBank Platform Guide

## Overview

**iBank** is MAPLE's platform for autonomous AI-only financial systems. With mandatory audit trails, risk assessments, digital signatures, and strict accountability, iBank enables safe autonomous financial decision-making up to $1M per transaction.

```
🏦 Autonomous AI Finance
🚫 No Human Participants
📜 Mandatory Audit Trails
⚖️ Risk Assessments Required
💰 $1M Autonomous Limit
✍️ Digital Signatures Required
```

## Core Characteristics

### 1. AI-Only Environment

**No human profiles allowed**:
- Only `IBank` profile Resonators
- Pure AI financial agents
- No human intervention in operations
- Optimized for algorithmic trading

### 2. Mandatory Audit Trails

**Every transaction fully audited**:
- Complete audit trail for all commitments
- Digital signatures (non-repudiation)
- Immutable records
- Regulatory compliance ready

### 3. Risk Assessment Required

**All financial actions assessed**:
- Risk score (0.0-1.0) calculated
- Risk factors identified
- Mitigation strategies required
- Financial impact estimated

### 4. Risk-Bounded Autonomy

**$1M autonomous decision limit**:
- Transactions ≤$1M fully autonomous
- Larger transactions require external approval
- Two-party confirmation for high-value operations
- Stop-loss and circuit breakers enforced

## Getting Started

### Create iBank Runtime

```rust
use maple_runtime::{MapleRuntime, config::ibank_runtime_config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Bootstrap iBank runtime
    let config = ibank_runtime_config();
    let runtime = MapleRuntime::bootstrap(config).await?;

    println!("✅ iBank runtime ready");

    // Your iBank application logic here

    runtime.shutdown().await?;
    Ok(())
}
```

### Register Financial Agent

```rust
use maple_runtime::{ResonatorSpec, ResonatorProfile};

// Create IBank Resonator
let mut spec = ResonatorSpec::default();
spec.profile = ResonatorProfile::IBank;
spec.display_name = Some("Trading Agent Alpha".to_string());
spec.capabilities = vec![
    Capability::Trading,
    Capability::RiskAssessment,
    Capability::PortfolioManagement,
];
spec.certifications = vec![
    Certification::FinancialRegulator,
    Certification::RiskManagement,
];

let financial_agent = runtime.register_resonator(spec).await?;
println!("🏦 Financial agent registered: {}", financial_agent.id);
```

## Configuration

### iBank Runtime Configuration

```rust
pub fn ibank_runtime_config() -> RuntimeConfig {
    RuntimeConfig {
        platform: Platform::IBank,

        profiles: ProfileConfig {
            human_profiles_allowed: false,  // AI-only
            allowed_profiles: vec![ResonatorProfile::IBank],
        },

        coupling: CouplingConfig {
            max_initial_strength: 0.3,
            max_strengthening_step: 0.1,
            require_explicit_intent: true,
            require_commitment_for_state_change: true,
        },

        commitment: CommitmentConfig {
            require_audit_trail: true,  // Mandatory
            require_digital_signature: true,  // Mandatory
            allow_best_effort: false,  // All commitments binding
            require_risk_assessment: true,  // Mandatory for finance
            immutable_audit_trail: true,  // Cannot be modified
        },

        attention: AttentionConfig {
            default_capacity: 2000.0,  // Higher for complex financial operations
            safety_reserve_pct: 0.2,  // Large safety margin
            exhaustion_threshold: 0.1,
            auto_rebalance: true,
        },

        financial: FinancialConfig {
            max_autonomous_value: 1_000_000.0,  // $1M limit
            require_two_party_threshold: 100_000.0,  // $100K
            stop_loss_required: true,
            circuit_breaker_enabled: true,
            max_drawdown: 0.2,  // 20%
        },

        consequence: ConsequenceConfig {
            maximum_autonomous_consequence_value: 1_000_000.0,
            require_reversibility_assessment: true,
            prefer_reversible: true,
        },

        temporal: TemporalConfig {
            anchor_retention: Duration::from_years(7),  // Regulatory requirement
            enable_vector_clocks: true,
            immutable_anchors: true,
        },
    }
}
```

## Core Patterns

### Pattern 1: Financial Transaction with Risk Assessment

```rust
// Execute trade with full risk assessment
let trade_commitment = financial_agent.create_commitment_with_risk(
    CommitmentContent::Action(ActionCommitment {
        action: "execute_trade".to_string(),
        parameters: hashmap!{
            "symbol" => "AAPL",
            "quantity" => 100,
            "side" => "BUY",
            "price" => 175.50,
            "value" => 17_550.0,
        },
        preconditions: vec![
            "market_open",
            "sufficient_capital",
            "within_risk_limits",
        ],
        postconditions: vec![
            "trade_executed",
            "position_updated",
            "audit_trail_complete",
        ],
        deadline: Some(now + 5_minutes),
    }),
    RiskAssessment {
        risk_score: 0.35,
        risk_factors: vec![
            RiskFactor {
                category: RiskCategory::Financial,
                score: 0.4,
                description: "Market volatility elevated".to_string(),
            },
            RiskFactor {
                category: RiskCategory::Operational,
                score: 0.3,
                description: "Depends on exchange API".to_string(),
            },
        ],
        financial_impact: Some(FinancialImpact {
            potential_loss: 3_510.0,  // 20% stop-loss
            potential_gain: 5_265.0,  // 30% target
            currency: "USD".to_string(),
        }),
        mitigations: vec![
            Mitigation {
                strategy: "Stop-loss at 2% below entry".to_string(),
                effectiveness: 0.95,
            },
            Mitigation {
                strategy: "Position sizing limited to 1% of portfolio".to_string(),
                effectiveness: 0.9,
            },
        ],
        assessed_by: financial_agent.id,
        assessed_at: TemporalAnchor::now(),
    }
).await?;

// Activate with digital signature
let signature = financial_agent.sign_commitment(&trade_commitment).await?;
trade_commitment.activate_with_signature(signature).await?;

// Execute trade
let execution_result = financial_agent.execute_trade(
    TradeParams {
        symbol: "AAPL",
        quantity: 100,
        side: TradeSide::Buy,
        order_type: OrderType::Limit,
        price: 175.50,
    }
).await?;

// Fulfill commitment with audit trail
trade_commitment.fulfill(
    hashmap!{
        "execution_price" => execution_result.price,
        "execution_time" => execution_result.timestamp,
        "order_id" => execution_result.order_id,
        "commission" => execution_result.commission,
    }
).await?;

println!("✅ Trade executed with full audit trail");
```

### Pattern 2: Two-Party Confirmation for Large Transactions

```rust
// Large transaction requiring two-party confirmation
let large_trade_value = 250_000.0;  // >$100K threshold

// First party creates commitment
let commitment = agent_1.create_commitment_with_risk(
    ActionCommitment {
        action: "execute_large_trade".to_string(),
        parameters: hashmap!{
            "value" => large_trade_value,
            // ...
        },
        // ...
    },
    risk_assessment
).await?;

// Mark as requiring two-party confirmation
commitment.require_two_party_confirmation().await?;

// Second party must review and confirm
let review = agent_2.review_commitment(commitment.id).await?;

if review.approved {
    // Second party signs
    let signature_2 = agent_2.sign_commitment(&commitment).await?;
    commitment.add_confirming_signature(signature_2).await?;

    // Now commitment can be activated
    commitment.activate().await?;

    // Execute with both parties' approval
    let result = execute_large_transaction(commitment).await?;

    commitment.fulfill(result).await?;
} else {
    // Second party rejected
    commitment.reject(review.reason).await?;
}
```

### Pattern 3: Risk-Bounded Portfolio Management

```rust
// Portfolio management with risk bounds
struct Portfolio {
    agent: ResonatorHandle,
    positions: Vec<Position>,
    total_value: f64,
    risk_limits: RiskLimits,
}

impl Portfolio {
    async fn rebalance(&mut self) -> Result<()> {
        // Assess current portfolio risk
        let current_risk = self.assess_portfolio_risk().await?;

        if current_risk.score > self.risk_limits.max_risk {
            // Portfolio exceeds risk limits - must rebalance

            // Create rebalancing commitment
            let commitment = self.agent.create_commitment_with_risk(
                CommitmentContent::Action(ActionCommitment {
                    action: "rebalance_portfolio".to_string(),
                    parameters: hashmap!{
                        "current_risk" => current_risk.score,
                        "target_risk" => self.risk_limits.target_risk,
                    },
                    // ...
                }),
                current_risk
            ).await?;

            commitment.activate().await?;

            // Execute rebalancing trades
            let trades = self.calculate_rebalancing_trades().await?;

            for trade in trades {
                // Each trade has its own commitment and risk assessment
                self.execute_trade_with_commitment(trade).await?;
            }

            // Fulfill rebalancing commitment
            commitment.fulfill(
                hashmap!{ "new_risk" => self.assess_portfolio_risk().await?.score }
            ).await?;
        }

        Ok(())
    }

    async fn enforce_stop_loss(&mut self, position: &Position) -> Result<()> {
        let current_price = get_current_price(position.symbol).await?;
        let loss_pct = (position.entry_price - current_price) / position.entry_price;

        if loss_pct > self.risk_limits.max_position_loss {
            // Stop-loss triggered - must exit position

            let commitment = self.agent.create_commitment_with_risk(
                CommitmentContent::Action(ActionCommitment {
                    action: "stop_loss_exit".to_string(),
                    parameters: hashmap!{
                        "symbol" => position.symbol.clone(),
                        "reason" => "stop_loss_triggered",
                        "loss_pct" => loss_pct,
                    },
                    // ...
                }),
                RiskAssessment {
                    risk_score: 0.9,  // High risk - losing position
                    // ...
                }
            ).await?;

            commitment.activate().await?;

            // Execute stop-loss
            let exit_result = self.exit_position(position).await?;

            commitment.fulfill(exit_result).await?;

            println!("🛑 Stop-loss executed: {} at {:.1}% loss",
                position.symbol, loss_pct * 100.0);
        }

        Ok(())
    }
}
```

### Pattern 4: Algorithmic Trading with Circuit Breakers

```rust
// Trading algorithm with circuit breakers
struct TradingAlgorithm {
    agent: ResonatorHandle,
    strategy: TradingStrategy,
    circuit_breaker: CircuitBreaker,
}

impl TradingAlgorithm {
    async fn execute_strategy(&mut self) -> Result<()> {
        loop {
            // Check circuit breaker status
            if self.circuit_breaker.is_open() {
                println!("⚠️ Circuit breaker open - trading halted");
                self.wait_for_circuit_breaker_reset().await?;
                continue;
            }

            // Generate trading signals
            let signals = self.strategy.generate_signals().await?;

            for signal in signals {
                // Check if trade within risk limits
                if !self.is_within_risk_limits(&signal).await? {
                    println!("⚠️ Signal {} exceeds risk limits - skipped", signal.id);
                    continue;
                }

                // Execute trade with commitment
                let result = self.execute_signal_with_commitment(signal).await?;

                // Update circuit breaker state
                self.circuit_breaker.record_trade(result).await?;

                // Check if circuit breaker should trigger
                if self.circuit_breaker.should_trigger().await? {
                    self.circuit_breaker.open().await?;
                    println!("🔴 Circuit breaker triggered - halting trading");
                    break;
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

struct CircuitBreaker {
    max_loss_per_hour: f64,
    max_consecutive_losses: usize,
    reset_duration: Duration,
    // ...
}

impl CircuitBreaker {
    async fn should_trigger(&self) -> Result<bool> {
        let recent_trades = self.get_recent_trades(Duration::from_hours(1)).await?;

        // Check hourly loss limit
        let total_loss: f64 = recent_trades.iter()
            .map(|t| t.profit_loss)
            .filter(|pl| *pl < 0.0)
            .sum();

        if total_loss.abs() > self.max_loss_per_hour {
            return Ok(true);
        }

        // Check consecutive losses
        let consecutive_losses = self.count_consecutive_losses(&recent_trades);

        if consecutive_losses >= self.max_consecutive_losses {
            return Ok(true);
        }

        Ok(false)
    }
}
```

## Use Cases

### 1. Autonomous Trading Systems

```rust
// Fully autonomous trading system
let trading_system = TradingSystem::new(
    agents: vec![
        create_market_data_agent().await?,
        create_signal_generator_agent().await?,
        create_execution_agent().await?,
        create_risk_manager_agent().await?,
    ],
    strategy: TradingStrategy::MeanReversion,
    risk_limits: RiskLimits {
        max_position_size: 50_000.0,
        max_portfolio_risk: 0.15,
        max_daily_loss: 10_000.0,
        // ...
    },
).await?;

// Run trading system with full audit trails
trading_system.start().await?;
```

### 2. AI-Managed Investment Portfolios

```rust
// Portfolio management AI
let portfolio_manager = PortfolioManager::new(
    profile: ResonatorProfile::IBank,
    mandate: InvestmentMandate {
        objective: "growth",
        risk_tolerance: RiskTolerance::Moderate,
        investment_horizon: Duration::from_years(5),
        // ...
    },
).await?;

// Manage portfolio autonomously
portfolio_manager.manage_portfolio().await?;
```

### 3. Decentralized Finance (DeFi)

```rust
// DeFi protocol agent
let defi_agent = DeFiAgent::new(
    protocols: vec![
        Protocol::UniswapV3,
        Protocol::Aave,
        Protocol::Compound,
    ],
).await?;

// Execute DeFi strategies with commitments
defi_agent.execute_strategy(
    Strategy::YieldFarming {
        pools: vec!["ETH/USDC", "BTC/USDC"],
        // ...
    }
).await?;
```

### 4. Algorithmic Market Making

```rust
// Market maker agent
let market_maker = MarketMaker::new(
    pairs: vec!["BTC/USD", "ETH/USD"],
    spread: 0.001,  // 0.1% spread
    inventory_limits: InventoryLimits {
        max_position: 100_000.0,
        // ...
    },
).await?;

// Provide liquidity with risk management
market_maker.start_market_making().await?;
```

## Audit and Compliance

### Generate Audit Report

```rust
// Comprehensive audit report
let report = runtime.generate_financial_audit_report(
    FinancialAuditRequest {
        agent: Some(financial_agent.id),
        time_range: (start_of_year, now),
        include_transactions: true,
        include_commitments: true,
        include_risk_assessments: true,
        include_signatures: true,
    }
).await?;

// Export for regulatory review
report.export_csv("ibank_audit_2026.csv")?;
report.export_pdf("ibank_audit_2026.pdf")?;

// Verify all signatures
let verification = report.verify_all_signatures().await?;
println!("Signatures verified: {}/{}",
    verification.valid_count, verification.total_count);
```

### Compliance Checking

```rust
// Check regulatory compliance
let compliance = runtime.check_financial_compliance(
    ComplianceCheck {
        standard: ComplianceStandard::SECRegulation,
        scope: ComplianceScope::AllTransactions,
        period: last_quarter,
    }
).await?;

if !compliance.compliant {
    println!("⚠️ Compliance violations found:");
    for violation in compliance.violations {
        println!("  - {}: {}", violation.rule, violation.description);
    }
}
```

## Risk Management

### Risk Assessment Framework

```rust
pub struct RiskAssessor {
    agent: ResonatorHandle,
}

impl RiskAssessor {
    pub async fn assess_trade_risk(&self, trade: &Trade) -> Result<RiskAssessment> {
        let mut risk_factors = Vec::new();

        // Market risk
        let market_risk = self.assess_market_risk(trade).await?;
        risk_factors.push(market_risk);

        // Liquidity risk
        let liquidity_risk = self.assess_liquidity_risk(trade).await?;
        risk_factors.push(liquidity_risk);

        // Operational risk
        let operational_risk = self.assess_operational_risk(trade).await?;
        risk_factors.push(operational_risk);

        // Credit risk (for margin trading)
        if trade.uses_margin {
            let credit_risk = self.assess_credit_risk(trade).await?;
            risk_factors.push(credit_risk);
        }

        // Calculate overall risk score
        let risk_score = self.calculate_overall_risk(&risk_factors);

        // Determine financial impact
        let financial_impact = self.estimate_financial_impact(trade, risk_score).await?;

        // Identify mitigation strategies
        let mitigations = self.identify_mitigations(trade, &risk_factors).await?;

        Ok(RiskAssessment {
            risk_score,
            risk_factors,
            financial_impact: Some(financial_impact),
            mitigations,
            assessed_by: self.agent.id,
            assessed_at: TemporalAnchor::now(),
        })
    }
}
```

## Best Practices

### For Financial Agent Developers

1. **Always assess risk before trading**
   ```rust
   let risk = assess_trade_risk(&trade).await?;
   if risk.risk_score > 0.7 {
       return Err(Error::RiskTooHigh);
   }
   ```

2. **Use stop-losses consistently**
   ```rust
   let trade_params = TradeParams {
       stop_loss: Some(entry_price * 0.98),  // 2% stop-loss
       // ...
   };
   ```

3. **Respect the $1M autonomous limit**
   ```rust
   if trade_value > MAX_AUTONOMOUS_VALUE {
       require_external_approval().await?;
   }
   ```

4. **Sign all commitments**
   ```rust
   let signature = agent.sign_commitment(&commitment).await?;
   commitment.activate_with_signature(signature).await?;
   ```

### For Platform Operators

1. **Monitor risk metrics**: Track overall system risk
2. **Review high-risk transactions**: Extra scrutiny for risk >0.7
3. **Audit regularly**: Quarterly compliance reviews
4. **Test circuit breakers**: Ensure fail-safes work
5. **Backup audit data**: 7-year retention required

## Summary

iBank provides **autonomous AI finance** with comprehensive safety:

- ✅ AI-only (no humans)
- ✅ Mandatory audit trails (immutable)
- ✅ Risk assessments required
- ✅ Digital signatures (non-repudiation)
- ✅ $1M autonomous limit
- ✅ Two-party confirmation for large transactions
- ✅ Stop-loss and circuit breakers
- ✅ Regulatory compliance ready

iBank enables safe autonomous financial decision-making at scale - the future of algorithmic finance.

## Related Documentation

- [Architecture Overview](../architecture.md) - System design
- [Profiles](../concepts/profiles.md) - IBank profile
- [Commitments](../concepts/commitments.md) - Accountability system
- [Mapleverse](mapleverse.md) - Pure AI platform

---

**Built with 🍁 by the MAPLE Team**
