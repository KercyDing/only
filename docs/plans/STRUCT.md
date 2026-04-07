# Only 项目结构设计

## 1. 结论

现阶段不建议一开始就拆成 `core`、`macro`、`cli`、`web` 多 crate 工作区。

推荐方案：

1. 先保持单一包结构，同时提供 `lib.rs` 和 `main.rs`。
2. 在 `src/` 内做清晰、稳定的模块分层。
3. 等核心边界被代码验证后，再按需要拆出独立 crate。

原因：

- 当前阶段的首要目标是把 `Onlyfile` 解析、任务选择、依赖规划、执行链路跑通。
- 白皮书锁定了功能边界，但还没有足够实现来证明哪些 API 真正稳定。
- 过早拆多 crate 会增加类型搬运、错误处理、测试装配和迭代成本。
- 但如果只有二进制入口，没有 `lib.rs`，测试和未来宿主复用也会被卡住。

所以，应该先做“单包内的 `lib.rs + main.rs` 混合结构稳定”，再做“多 crate 物理拆分”。

这和 `just` 的成熟模式一致：

- `src/lib.rs` 暴露核心逻辑，便于单元测试、集成测试和未来复用。
- `src/main.rs` 保持极薄，只负责命令行入口和退出码桥接。

## 2. 对 `core / macro / cli / web` 的判断

### 2.1 `cli`

不需要独立 crate，但需要单独模块。

- `only` 本身就是 CLI 应用。
- `clap` 路由和参数分发属于入口层，不是高价值复用库。
- 放在 `src/cli/` 足够清晰。
- `main.rs` 只负责调用 `cli` 和 `lib.rs` 暴露的主流程。

### 2.2 `core`

现在不建议物理拆 crate，但要在逻辑上形成核心层，并通过 `lib.rs` 对外暴露。

- 语言模型、解析器、规划器、运行时确实是项目核心。
- 但 MVP 期间这些接口还会频繁调整。
- 现在就拆 `only-core`，容易过早冻结 API。

建议先在单包内部划分 `model`、`parser`、`planner`、`runtime`，再由 `lib.rs` 统一导出公共 API。

### 2.3 `macro`

当前不应存在。

- 白皮书没有过程宏需求。
- 解析 `Onlyfile` 不依赖 Rust 宏系统。
- 为了“以后可能用到”而建 `macro` crate，属于过度设计。

### 2.4 `web`

当前不应进入 MVP 主体结构。

- 白皮书明确 `only --web` 属于 `v1.0`，不是 MVP。
- Web 是核心能力之上的表现层，不应反向主导核心架构。
- 现在只需要在设计上预留扩展点，不需要实现目录或 crate。

## 3. 推荐的第一阶段目录结构

```text
only/
├── Cargo.toml
├── README.md
├── docs/
│   └── plans/
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── app.rs
│   │   ├── args.rs
│   │   └── dispatch.rs
│   ├── config/
│   │   ├── mod.rs
│   │   └── discover.rs
│   ├── model/
│   │   ├── mod.rs
│   │   ├── directive.rs
│   │   ├── namespace.rs
│   │   ├── probe.rs
│   │   ├── span.rs
│   │   └── task.rs
│   ├── parser/
│   │   ├── mod.rs
│   │   ├── lexer.rs
│   │   ├── grammar.rs
│   │   └── validate.rs
│   ├── planner/
│   │   ├── mod.rs
│   │   ├── dag.rs
│   │   └── resolve.rs
│   ├── runtime/
│   │   ├── mod.rs
│   │   ├── engine.rs
│   │   ├── interpolate.rs
│   │   ├── probe.rs
│   │   └── process.rs
│   ├── diagnostic/
│   │   ├── mod.rs
│   │   └── error.rs
│   └── support/
│       ├── mod.rs
│       ├── fs.rs
│       └── path.rs
└── tests/
    ├── fixtures/
    ├── parse.rs
    ├── planner.rs
    └── runtime.rs
```

核心思路：按职责流分层，不按空泛概念分层。

执行链路：

1. `cli` 接收参数。
2. `config` 发现 `Onlyfile`。
3. `parser` 把文本转成 `model`。
4. `planner` 做任务定位、守卫匹配、依赖展开和环检测。
5. `runtime` 做参数注入和命令执行。
6. `diagnostic` 统一承载错误输出。

## 4. 各层职责

### 4.1 `main.rs`

- 只保留程序入口。
- 解析 CLI 参数。
- 调用 `lib.rs` 暴露的主流程。
- 统一退出码。

### 4.2 `lib.rs`

- 组织内部模块。
- 暴露最小可复用 API，供测试、集成测试和未来前端宿主调用。

建议对外暴露：

- `load_onlyfile`
- `parse_onlyfile`
- `build_execution_plan`
- `run_plan`

`lib.rs` 的目标不是把所有内部细节都公开，而是提供稳定、可测试、可复用的核心入口。

### 4.3 `cli/`

- 定义命令行形态。
- 解析用户输入。
- 调用核心流程。

建议：

- `app.rs` 负责 `clap::Command`。
- `args.rs` 表达归一化后的 CLI 输入。
- `dispatch.rs` 负责把 CLI 输入转成核心调用。

### 4.4 `config/`

- 查找 `Onlyfile` 或 `onlyfile`。
- 管理文件路径上下文。
- 为未来 `!env_file`、`!working_dir` 预留入口。

### 4.5 `model/`

- 定义稳定领域对象。
- 不依赖 CLI、具体 parser、具体 shell。

建议核心类型：

- `Onlyfile`
- `Directive`
- `Namespace`
- `TaskDefinition`
- `TaskSignature`
- `Parameter`
- `Guard`
- `ProbeCall`
- `CommandLine`
- `Span`

### 4.6 `parser/`

- 词法分析。
- 语法分析。
- 基础语义校验。

建议分成：

- `lexer.rs`: `logos`
- `grammar.rs`: `winnow`
- `validate.rs`: 重复定义、非法顺序、命名冲突、未定义引用等

这也对应附录 B 的词法错误、语法错误、语义错误分层。

### 4.7 `planner/`

- 根据 CLI 选择目标任务。
- 处理命名空间默认任务。
- 处理同名任务的守卫匹配。
- 构建依赖 DAG。
- 检测循环依赖。
- 输出执行计划。

这一层对应白皮书中的 Phase 1。

### 4.8 `runtime/`

- 参数绑定。
- `{{}}` 插值替换。
- 运行命令。
- 传播退出码。
- 处理 `!verbose`。

这一层对应白皮书中的 Phase 2。

### 4.9 `diagnostic/`

- 统一错误类型。
- 承载 `miette` 诊断。
- 提供带位置信息的解析和语义报错。

### 4.10 `support/`

- 放通用但不属于核心领域的工具代码。
- 例如路径规范化、少量文件系统辅助逻辑。

原则：不要让 `support/` 变成杂物间，只有明确跨模块复用的非领域代码才能进入这里。

## 5. 为什么不建议所有代码都塞进 `src/`

如果把所有逻辑直接堆在 `src/main.rs` 或少数几个文件里，短期写得快，长期一定失控：

- 解析、规划、执行三段模型会混在一起。
- 错误处理会散落到各处。
- 单元测试和集成测试只能绕二进制入口，测试成本更高。
- 后续要做 `--help` 动态生成、`--dry-run`、`--web` 时会被迫重构。

所以答案不是“全部写在 `src` 里一个文件”，而是“保留 `lib.rs + main.rs`，并在同一包的 `src/` 下严格模块化”。

## 6. 未来何时拆成多 crate

满足以下条件后，再考虑拆分：

1. `lib.rs` 暴露的核心 API 基本稳定。
2. `model + parser + planner` 的内部边界已经清晰。
3. 需要被多个宿主复用，例如 CLI、Web、TUI。
4. 测试已经证明跨层边界清晰。

届时建议的演进顺序：

1. 先拆 `only-core`
2. 再让当前二进制依赖 `only-core`
3. 最后按需要新增 `only-web`

仍然不建议单独创建 `only-macro`，除非真的出现稳定且必要的宏能力。

## 7. 实施建议

第一批代码建议按这个顺序落地：

1. `model`
2. `config/discover`
3. `parser`
4. `planner`
5. `cli`
6. `runtime`

这样做可以先把“能解析并决定执行什么”跑通，再接上“如何执行”。

## 8. 最终建议

当前阶段的最佳答案是：

- 不做 `core / macro / cli / web` 多 crate 分层。
- 做单包下的 `lib.rs + main.rs` 混合结构。
- 以 `model / parser / planner / runtime / cli` 为主结构。
- 为未来 `only-core` 和 `only-web` 预留演进路径，但不提前实现。
