//! Application, Web & Enterprise Polyglot Drivers Barrel.
//!
//! Exposes lossless drivers for TypeScript/JavaScript, Dart/Flutter, Python, PHP, and Java,
//! along with the Component Tree Extractor and State Dataflow Hazard Tracker.

#![forbid(unsafe_code)]

pub mod component_tree;
pub mod dart_driver;
pub mod dataflow;
pub mod java_driver;
pub mod php_driver;
pub mod python_driver;
pub mod ts_driver;

pub use component_tree::{
    ComponentNode, ComponentProp, ComponentTreeExtractor, ComponentType, LayoutRole,
};
pub use dart_driver::DartDriver;
pub use dataflow::{
    DataflowHazard, DataflowReport, DataflowTracker, StateDataflowGraph, StateMutation, StateRead,
    StateSource, StateSourceKind,
};
pub use java_driver::JavaDriver;
pub use php_driver::PhpDriver;
pub use python_driver::PythonDriver;
pub use ts_driver::TypeScriptDriver;

use crate::cst::polyglot::traits::UniversalCstDriver;
use crate::cst::polyglot::types::LanguageId;

/// Factory function to retrieve the appropriate polyglot CST driver for a language identifier.
pub fn get_app_driver(lang: &LanguageId) -> Option<Box<dyn UniversalCstDriver>> {
    match lang {
        LanguageId::TypeScript
        | LanguageId::JavaScript
        | LanguageId::Tsx
        | LanguageId::Jsx
        | LanguageId::Svelte
        | LanguageId::Astro
        | LanguageId::Vue => Some(Box::new(TypeScriptDriver::with_language(lang.clone()))),
        LanguageId::Dart => Some(Box::new(DartDriver::new())),
        LanguageId::Python => Some(Box::new(PythonDriver::new())),
        LanguageId::Php => Some(Box::new(PhpDriver::new())),
        LanguageId::Java => Some(Box::new(JavaDriver::new())),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Comprehensive Unit & Invariant Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Language, SymbolKind};
    use std::time::Instant;

    // --- 1. Lossless Roundtrip Tests Across All 5 Languages ---

    #[test]
    fn test_ts_driver_lossless_roundtrip() {
        let source = r#"
import React, { useState, useEffect } from 'react';
import type { UserProfile } from './types';

/** User profile card component */
@Component({ selector: 'app-user-card' })
export class UserCardComponent implements OnInit {
    private isLoaded: boolean = false;

    async ngOnInit(): Promise<void> {
        const [user, setUser] = useState<UserProfile | null>(null);
    }
}

export const HeaderBar = ({ title }: { title: string }) => {
    return (
        <div className="header-container flex flex-row">
            <h1>{title}</h1>
            <button onClick={() => console.log('clicked')}>Action</button>
        </div>
    );
};
"#;
        let driver = TypeScriptDriver::new();
        let tree = driver.parse_lossless(source).expect("TS parsing must succeed");
        assert_eq!(tree.source_lossless_text(), source, "TS CST must match original 100% byte-for-byte");

        let symbols = driver.extract_symbols(&tree);
        println!("TS symbols: {:?}", symbols);
        assert!(symbols.iter().any(|s| s.name == "UserCardComponent" && s.kind == SymbolKind::Struct));
        assert!(symbols.iter().any(|s| s.name == "HeaderBar" && s.kind == SymbolKind::Function));

        let cfg = driver.compute_cfg(&tree);
        assert!(cfg.node_count() >= 2, "CFG must generate nodes for basic blocks");
    }

    #[test]
    fn test_dart_driver_lossless_roundtrip() {
        let source = r#"
import 'package:flutter/material.dart';
import 'package:provider/provider.dart' as provider;

/// Main counter widget representation
class CounterView extends StatefulWidget {
    final String title;
    const CounterView({super.key, required this.title});

    @override
    State<CounterView> createState() => _CounterViewState();
}

class _CounterViewState extends State<CounterView> {
    int _count = 0;

    void _increment() {
        setState(() {
            _count++;
        });
    }

    @override
    Widget build(BuildContext context) {
        return Scaffold(
            appBar: AppBar(title: Text(widget.title)),
            body: Center(
                child: Column(
                    children: [
                        Text('Count: $_count'),
                        ElevatedButton(onPressed: _increment, child: Text('Add')),
                    ],
                ),
            ),
        );
    }
}
"#;
        let driver = DartDriver::new();
        let tree = driver.parse_lossless(source).expect("Dart parsing must succeed");
        assert_eq!(tree.source_lossless_text(), source, "Dart CST must match original 100% byte-for-byte");

        let symbols = driver.extract_symbols(&tree);
        eprintln!("DART SYMBOLS: {:#?}", symbols);
        assert!(symbols.iter().any(|s| s.name == "CounterView" && s.kind == SymbolKind::Struct));
        assert!(symbols.iter().any(|s| s.name == "build" && s.kind == SymbolKind::Function));
    }

    #[test]
    fn test_python_driver_lossless_roundtrip() {
        let source = r#"
from typing import List, Optional, Dict
import asyncio

"""Module documentation for async telemetry handler."""

@dataclass
class TelemetryPacket:
    device_id: str
    metrics: Dict[str, float]

@app.get("/metrics")
async def collect_metrics(packets: List[TelemetryPacket], timeout: float = 5.0) -> bool:
    """Collects and aggregates metric packets."""
    for p in packets:
        await process_packet(p)
    return True
"#;
        let driver = PythonDriver::new();
        let tree = driver.parse_lossless(source).expect("Python parsing must succeed");
        assert_eq!(tree.source_lossless_text(), source, "Python CST must match original 100% byte-for-byte");

        let symbols = driver.extract_symbols(&tree);
        assert!(symbols.iter().any(|s| s.name == "TelemetryPacket" && s.kind == SymbolKind::Struct));
        assert!(symbols.iter().any(|s| s.name == "collect_metrics" && s.kind == SymbolKind::Function));
    }

    #[test]
    fn test_php_driver_lossless_roundtrip() {
        let source = r#"<?php

namespace App\Http\Controllers;

use App\Models\User;
use App\Services\AuthService;
use Illuminate\Http\JsonResponse;

#[Route('/api/v1/auth', name: 'api_auth')]
class AuthController extends BaseController {
    public function __construct(
        private readonly AuthService $authService
    ) {}

    #[Route('/login', methods: ['POST'])]
    public function login(Request $request): JsonResponse {
        $data = $request->validate();
        return response()->json($this->authService->authenticate($data));
    }
}
"#;
        let driver = PhpDriver::new();
        let tree = driver.parse_lossless(source).expect("PHP parsing must succeed");
        assert_eq!(tree.source_lossless_text(), source, "PHP CST must match original 100% byte-for-byte");

        let symbols = driver.extract_symbols(&tree);
        assert!(symbols.iter().any(|s| s.name == "App\\Http\\Controllers" && s.kind == SymbolKind::Module));
        assert!(symbols.iter().any(|s| s.name == "AuthController" && s.kind == SymbolKind::Struct));
        assert!(symbols.iter().any(|s| s.name == "login" && s.kind == SymbolKind::Function));
    }

    #[test]
    fn test_java_driver_lossless_roundtrip() {
        let source = r#"
package com.locus.enterprise.order;

import java.util.List;
import java.util.UUID;
import org.springframework.web.bind.annotation.*;

/**
 * Enterprise order processing controller.
 */
@RestController
@RequestMapping("/api/v1/orders")
public class OrderController {
    private final OrderService orderService;

    public OrderController(OrderService orderService) {
        this.orderService = orderService;
    }

    @PostMapping
    public OrderResponse createOrder(@RequestBody OrderRequest req) {
        return this.orderService.process(req);
    }
}

public record OrderResponse(UUID orderId, String status, double totalAmount) {}
"#;
        let driver = JavaDriver::new();
        let tree = driver.parse_lossless(source).expect("Java parsing must succeed");
        assert_eq!(tree.source_lossless_text(), source, "Java CST must match original 100% byte-for-byte");

        let symbols = driver.extract_symbols(&tree);
        assert!(symbols.iter().any(|s| s.name == "com.locus.enterprise.order" && s.kind == SymbolKind::Module));
        assert!(symbols.iter().any(|s| s.name == "OrderController" && s.kind == SymbolKind::Struct));
        assert!(symbols.iter().any(|s| s.name == "OrderResponse" && s.kind == SymbolKind::Struct));
    }

    // --- 2. Component Tree Extractor Tests (React & Flutter) ---

    #[test]
    fn test_react_tsx_component_tree_extraction() {
        let source = r#"
export function DashboardLayout() {
    return (
        <div className="flex flex-col container">
            <Header title="Mission Control" />
            <div className="grid grid-cols-2">
                <Card title="Telemetry">
                    <p>Status: Active</p>
                </Card>
                <Card title="Logs">
                    <button onClick={() => handleRefresh()}>Refresh</button>
                </Card>
            </div>
        </div>
    );
}
"#;
        let extractor = ComponentTreeExtractor::new();
        let trees = extractor.extract(source, Language::Tsx);
        assert!(!trees.is_empty(), "Should extract JSX root node");
        let total = extractor.total_nodes(&trees);
        assert!(total >= 5, "Should extract nested tree hierarchy (total >= 5, got {})", total);

        let cards = extractor.find_by_name(&trees, "Card");
        assert_eq!(cards.len(), 2, "Should find 2 Card components");
    }

    #[test]
    fn test_flutter_widget_tree_extraction() {
        let source = r#"
Widget build(BuildContext context) {
    return Scaffold(
        appBar: AppBar(title: Text("Dashboard")),
        body: Center(
            child: Column(
                children: [
                    Text("Telemetry Stream"),
                    ElevatedButton(
                        onPressed: () => handleSync(),
                        child: Text("Sync Now"),
                    ),
                ],
            ),
        ),
    );
}
"#;
        let extractor = ComponentTreeExtractor::new();
        let trees = extractor.extract(source, Language::Dart);
        assert!(!trees.is_empty(), "Should extract Flutter widget root");
        let scaffolds = extractor.find_by_name(&trees, "Scaffold");
        assert!(!scaffolds.is_empty(), "Should find Scaffold widget");
        let columns = extractor.find_by_name(&trees, "Column");
        assert!(!columns.is_empty(), "Should find Column widget with LayoutRole::Column");
        if let Some(col) = columns.first() {
            assert_eq!(col.layout_role, LayoutRole::Column);
        }
    }

    // --- 3. State & Dataflow Tracking & Hazard Detection ---

    #[test]
    fn test_dataflow_state_race_hazard_detection() {
        let hazardous_code = r#"
export function AsyncCounter() {
    const [count, setCount] = useState(0);

    const handleBadAsyncUpdate = async () => {
        const res = await fetch('/api/data');
        // Non-functional state update in async scope => HAZARD!
        setCount(count + 1);
    };

    const handleSafeUpdate = async () => {
        // Functional update is safe
        setCount(prev => prev + 1);
    };

    return <div>{count}</div>;
}
"#;
        let tracker = DataflowTracker::new();
        let report = tracker.analyze(hazardous_code, Language::Tsx);
        assert_eq!(report.total_state_variables, 1);
        assert_eq!(report.graph.sources[0].name, "count");
        assert!(report.hazard_count > 0, "Should detect ASYNC_NON_FUNCTIONAL_STATE_UPDATE hazard");
        assert!(report.graph.hazards.iter().any(|h| h.hazard_type == "ASYNC_NON_FUNCTIONAL_STATE_UPDATE"));
    }

    #[test]
    fn test_dataflow_java_atomic_flow() {
        let code = r#"
public class ConcurrentWorker {
    private final AtomicReference<AppState> state = new AtomicReference<>();

    public void update() {
        state.set(new AppState());
    }
}
"#;
        let tracker = DataflowTracker::new();
        let report = tracker.analyze(code, Language::Java);
        assert_eq!(report.total_state_variables, 1);
        assert_eq!(report.graph.sources[0].name, "state");
        assert!(report.graph.sources[0].is_concurrent_safe);
        assert_eq!(report.total_mutations, 1);
    }

    // --- 4. Sub-0.05ms Performance Budget Verification ---

    #[test]
    fn test_sub_0_05ms_parsing_budget() {
        let ts_code = r#"
import { useState } from 'react';
export const Card = ({ id, name }: { id: string; name: string }) => {
    const [active, setActive] = useState(false);
    return <div onClick={() => setActive(!active)}><span>{name}</span></div>;
};
"#;
        let driver = TypeScriptDriver::new();

        // Warm up
        for _ in 0..10 {
            let _ = driver.parse_lossless(ts_code);
        }

        let iterations = 1000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _tree = driver.parse_lossless(ts_code).expect("Parsing succeeds");
        }
        let total_time_ms = start.elapsed().as_secs_f64() * 1000.0;
        let avg_latency_ms = total_time_ms / (iterations as f64);

        println!("⚡ Average Polyglot CST Parsing Latency: {:.4}ms", avg_latency_ms);
        let budget = if cfg!(debug_assertions) { 0.50 } else { 0.05 };
        assert!(
            avg_latency_ms < budget,
            "Parsing latency must satisfy the budget ({:.2}ms), got {:.4}ms",
            budget,
            avg_latency_ms
        );
    }
}
