# Language-Specific Workflow Templates

This directory contains workflow templates tailored for specific programming languages and frameworks. Each language has curated workflows for common development tasks.

## Table of Contents

- [Validation Workflows](#validation-workflows) ⭐ NEW
  - [Pre-Commit Validation](#pre-commit-validation)
  - [CI/CD Validation](#cicd-validation)
- [Compiled Languages](#compiled-languages)
  - [C](#c)
  - [C++](#c-1)
  - [Java](#java)
  - [Fortran](#fortran)
- [Interpreted Languages](#interpreted-languages)
  - [Python](#python)
  - [JavaScript](#javascript)
  - [TypeScript](#typescript)
  - [Lisp](#lisp)
- [Web Frameworks](#web-frameworks)
  - [React.js](#reactjs)
  - [Angular](#angular)
  - [SvelteKit](#sveltekit)
- [Markup & Data](#markup--data)
  - [HTML](#html)
  - [Schema Validation](#schema-validation)
- [Scripting & Database](#scripting--database)
  - [Shell Scripts](#shell-scripts)
  - [SQL](#sql)

---

## Validation Workflows

⭐ **NEW**: Comprehensive validation workflows that combine multiple quality checks!

### Pre-Commit Validation

**Location:** `/workflows/validation/`

Fast validation for pre-commit hooks - checks only staged files.

| Workflow | ID | Purpose |
|----------|-----|---------|
| `pre_commit.yaml` | `pre_commit_validate` | Fast pre-commit checks (<30s) on staged files |

**Features:**
- Language detection (Python, TypeScript, JavaScript, C/C++, etc.)
- Quick linting on changed files only
- Format checking
- Secret detection
- Merge conflict detection
- Blocks critical issues, warns on minor ones

**Usage:**
```bash
aco workflow execute pre_commit_validate --input "Check staged files"
```

**Git Hook Integration:**
```bash
# .git/hooks/pre-commit
#!/bin/bash
aco workflow execute pre_commit_validate --input "Pre-commit validation"
```

---

### CI/CD Validation

**Location:** `/workflows/validation/`

Comprehensive validation for CI/CD pipelines - full test suite and all checks.

| Workflow | ID | Purpose |
|----------|-----|---------|
| `ci_cd.yaml` | `ci_cd_validate` | Full CI/CD pipeline validation |

**Features:**
- Auto-detect project type and language
- Run language-specific validation workflows
- Security scanning (dependency audit, secret detection, SAST)
- Full test suite with coverage
- Build verification
- Integration tests
- Performance checks
- Comprehensive quality gates

**Usage:**
```bash
aco workflow execute ci_cd_validate --input "Full CI validation"
aco workflow execute ci_cd_validate --input "PR validation check"
```

**Quality Gates:**
- ✅ All tests pass
- ✅ Coverage >= 80%
- ✅ No critical security issues
- ✅ Build succeeds
- ✅ Lint score >= 9/10

---

## Compiled Languages

### C

**Location:** `/workflows/languages/c/`

Workflows for C projects using gcc/clang/make/cmake.

| Workflow | ID | Purpose |
|----------|-----|---------|
| `build.yaml` | `c_build` | Compile C project with gcc/clang/make/cmake |
| `test.yaml` | `c_test` | Run C unit tests (Check/CTest/Unity) |
| `debug_build.yaml` | `c_debug_build` | Debug and fix compilation errors |
| `validate.yaml` ⭐ | `c_validate` | Full validation: build + test + static analysis |

**Usage:**
```bash
aco workflow execute c_build --input "Build the project"
aco workflow execute c_test --input "Run all tests"
aco workflow execute c_debug_build --input "Fix compilation errors"
aco workflow execute c_validate --input "Full validation pipeline"
```

---

### C++

**Location:** `/workflows/languages/cpp/`

Workflows for C++ projects using g++/clang++/cmake.

| Workflow | ID | Purpose |
|----------|-----|---------|
| `build.yaml` | `cpp_build` | Compile C++ project |
| `test.yaml` | `cpp_test` | Run tests (Google Test/Catch2/CTest) |
| `debug_build.yaml` | `cpp_debug_build` | Debug compilation errors |
| `validate.yaml` ⭐ | `cpp_validate` | Full validation: build + test + static analysis |

**Usage:**
```bash
aco workflow execute cpp_build --input "Build with C++17"
aco workflow execute cpp_test --input "Run unit tests"
```

---

### Java

**Location:** `/workflows/languages/java/`

Workflows for Java projects using Maven/Gradle.

| Workflow | ID | Purpose |
|----------|-----|---------|
| `build.yaml` | `java_build` | Build Java project (Maven/Gradle) |
| `test.yaml` | `java_test` | Run JUnit tests |
| `debug_build.yaml` | `java_debug_build` | Fix compilation errors |
| `validate.yaml` ⭐ | `java_validate` | Full validation: build + test + coverage + checkstyle |

**Usage:**
```bash
aco workflow execute java_build --input "Build with Maven"
aco workflow execute java_test --input "Run all JUnit tests"
```

---

### Fortran

**Location:** `/workflows/languages/fortran/`

Workflows for Fortran projects.

| Workflow | ID | Purpose |
|----------|-----|---------|
| `build.yaml` | `fortran_build` | Compile Fortran (gfortran/ifort) |
| `test.yaml` | `fortran_test` | Run Fortran tests |

**Usage:**
```bash
aco workflow execute fortran_build --input "Compile with gfortran"
```

---

## Interpreted Languages

### Python

**Location:** `/workflows/languages/python/`

Comprehensive workflows for Python development.

| Workflow | ID | Purpose |
|----------|-----|---------|
| `build.yaml` | `python_build` | Build/install Python package |
| `test.yaml` | `python_test` | Run tests with pytest/unittest |
| `lint.yaml` | `python_lint` | Lint with ruff/pylint/flake8/mypy |
| `format.yaml` | `python_format` | Format with black/ruff |
| `debug_runtime.yaml` | `python_debug_runtime` | Debug runtime errors |
| `validate.yaml` ⭐ | `python_validate` | **FULL VALIDATION**: lint + typecheck + security + test + build |

**Usage:**
```bash
aco workflow execute python_build --input "Build package"
aco workflow execute python_test --input "Run tests with coverage"
aco workflow execute python_lint --input "Lint and fix issues"
aco workflow execute python_format --input "Format all code"
aco workflow execute python_debug_runtime --input "Debug AttributeError"
```

---

### JavaScript

**Location:** `/workflows/languages/javascript/`

Workflows for JavaScript projects.

| Workflow | ID | Purpose |
|----------|-----|---------|
| `build.yaml` | `javascript_build` | Bundle with Vite/Webpack/Rollup |
| `test.yaml` | `javascript_test` | Run tests (Jest/Vitest/Mocha) |
| `lint.yaml` | `javascript_lint` | Lint with ESLint |
| `validate.yaml` ⭐ | `javascript_validate` | Full validation: lint + test + build |

**Usage:**
```bash
aco workflow execute javascript_build --input "Build for production"
aco workflow execute javascript_test --input "Run all tests"
aco workflow execute javascript_lint --input "Lint and auto-fix"
```

---

### TypeScript

**Location:** `/workflows/languages/typescript/`

Workflows for TypeScript projects.

| Workflow | ID | Purpose |
|----------|-----|---------|
| `build.yaml` | `typescript_build` | Compile TypeScript (tsc/webpack/vite) |
| `test.yaml` | `typescript_test` | Run tests (Jest/Vitest) |
| `lint.yaml` | `typescript_lint` | Lint with ESLint |
| `typecheck.yaml` | `typescript_typecheck` | Type checking with tsc --noEmit |
| `validate.yaml` ⭐ | `typescript_validate` | **FULL VALIDATION**: typecheck + lint + format + test + build |

**Usage:**
```bash
aco workflow execute typescript_build --input "Build project"
aco workflow execute typescript_test --input "Run tests with coverage"
aco workflow execute typescript_lint --input "Lint TypeScript code"
aco workflow execute typescript_typecheck --input "Run type checking"
```

---

### Lisp

**Location:** `/workflows/languages/lisp/`

Workflows for Common Lisp projects.

| Workflow | ID | Purpose |
|----------|-----|---------|
| `build.yaml` | `lisp_build` | Build with SBCL/CCL/ASDF |
| `test.yaml` | `lisp_test` | Run tests (FiveAM/lisp-unit) |
| `repl_debug.yaml` | `lisp_repl_debug` | Interactive REPL debugging |

**Usage:**
```bash
aco workflow execute lisp_build --input "Build Lisp system"
aco workflow execute lisp_test --input "Run test suite"
```

---

## Web Frameworks

### React.js

**Location:** `/workflows/languages/reactjs/`

Workflows for React.js applications.

| Workflow | ID | Purpose |
|----------|-----|---------|
| `dev.yaml` | `reactjs_dev` | Start development server |
| `build.yaml` | `reactjs_build` | Build for production |
| `test.yaml` | `reactjs_test` | Run component tests |
| `lint.yaml` | `reactjs_lint` | Lint with ESLint + React rules |
| `validate.yaml` ⭐ | `reactjs_validate` | Full validation: lint + typecheck + test + build + bundle size |

**Usage:**
```bash
aco workflow execute reactjs_build --input "Build React app"
aco workflow execute reactjs_test --input "Run component tests"
```

---

### Angular

**Location:** `/workflows/languages/angular/`

Workflows for Angular applications.

| Workflow | ID | Purpose |
|----------|-----|---------|
| `dev.yaml` | `angular_dev` | Start ng serve |
| `build.yaml` | `angular_build` | Build for production |
| `test.yaml` | `angular_test` | Run tests (Jasmine/Karma) |
| `lint.yaml` | `angular_lint` | Lint with ESLint |
| `validate.yaml` ⭐ | `angular_validate` | Full validation: lint + test + build + AOT |

**Usage:**
```bash
aco workflow execute angular_build --input "Build Angular app"
aco workflow execute angular_test --input "Run unit tests"
```

---

### SvelteKit

**Location:** `/workflows/languages/sveltekit/`

Workflows for SvelteKit applications.

| Workflow | ID | Purpose |
|----------|-----|---------|
| `dev.yaml` | `sveltekit_dev` | Start dev server |
| `build.yaml` | `sveltekit_build` | Build for production |
| `test.yaml` | `sveltekit_test` | Run tests (Vitest/Playwright) |
| `check.yaml` | `sveltekit_check` | Type checking with svelte-check |
| `validate.yaml` ⭐ | `sveltekit_validate` | Full validation: svelte-check + test + build |

**Usage:**
```bash
aco workflow execute sveltekit_build --input "Build SvelteKit app"
aco workflow execute sveltekit_check --input "Run type checking"
```

---

## Markup & Data

### HTML

**Location:** `/workflows/languages/html/`

Workflows for HTML validation and formatting.

| Workflow | ID | Purpose |
|----------|-----|---------|
| `validate.yaml` | `html_validate` | Validate HTML syntax (W3C) |
| `lint.yaml` | `html_lint` | Lint with HTMLHint |
| `format.yaml` | `html_format` | Format with Prettier |

**Usage:**
```bash
aco workflow execute html_validate --input "Validate HTML files"
aco workflow execute html_lint --input "Lint HTML"
```

---

### Schema Validation

**Location:** `/workflows/languages/schema/`

Workflows for validating data schemas.

| Workflow | ID | Purpose |
|----------|-----|---------|
| `validate_json.yaml` | `json_schema_validate` | Validate JSON against schema |
| `validate_xml.yaml` | `xml_schema_validate` | Validate XML against XSD |
| `validate_yaml.yaml` | `yaml_schema_validate` | Validate YAML syntax/schema |

**Usage:**
```bash
aco workflow execute json_schema_validate --input "Validate data.json"
aco workflow execute xml_schema_validate --input "Validate against XSD"
aco workflow execute yaml_schema_validate --input "Validate YAML config"
```

---

## Scripting & Database

### Shell Scripts

**Location:** `/workflows/languages/shell/`

Workflows for shell script validation.

| Workflow | ID | Purpose |
|----------|-----|---------|
| `lint.yaml` | `shell_lint` | Lint with ShellCheck |
| `validate.yaml` | `shell_validate` | Syntax validation |
| `test.yaml` | `shell_test` | Run tests (Bats/shunit2) |

**Usage:**
```bash
aco workflow execute shell_lint --input "Lint shell scripts"
aco workflow execute shell_validate --input "Check syntax"
```

---

### SQL

**Location:** `/workflows/languages/sql/`

Workflows for SQL validation and execution.

| Workflow | ID | Purpose |
|----------|-----|---------|
| `validate.yaml` | `sql_validate` | Validate SQL syntax |
| `lint.yaml` | `sql_lint` | Lint with sqlfluff |
| `execute.yaml` | `sql_execute` | Safe SQL execution |

**Usage:**
```bash
aco workflow execute sql_validate --input "Validate query.sql"
aco workflow execute sql_lint --input "Lint SQL files"
```

---

## Workflow Patterns

All workflows use proven AI agent patterns:

- **ReAct Pattern** (Reasoning + Acting): Most workflows use this iterative pattern
  - Think → Act → Observe cycle
  - Self-correcting through iterations
  - Ideal for build, test, debug tasks

- **Plan-Execute Pattern**: For complex multi-step tasks
  - Plan → Execute → Replan cycle
  - Used for larger refactoring tasks

- **Reflection Pattern**: For code quality improvements
  - Generate → Critique → Refine cycle

---

## General Usage

### Execute a Workflow

```bash
aco workflow execute <workflow_id> --input "<task description>"
```

### List Available Workflows

```bash
aco workflow list
```

### View Workflow Details

```bash
aco workflow show <workflow_id>
```

---

## Workspace Configuration

Each language has a corresponding workspace configuration template in `/templates/languages/`:

- Copy to `.orca/workspace.yaml` in your project
- Customize for your specific needs
- Defines allowed tools, file types, build commands

Example:
```bash
cp templates/languages/python_workspace.yaml .orca/workspace.yaml
```

---

## Contributing

To add a new language workflow:

1. Create directory: `workflows/languages/<language>/`
2. Add workflow YAML files (build, test, etc.)
3. Create workspace template: `templates/languages/<language>_workspace.yaml`
4. Update this README with documentation

---

## Summary Statistics

**Languages Supported:** 15
- C, C++, Java, Fortran (Compiled)
- Python, JavaScript, TypeScript, Lisp (Interpreted)
- React, Angular, SvelteKit (Frameworks)
- HTML, Schema, Shell, SQL (Utilities)

**Total Workflows:** 72+
- Individual workflows: 61
- **NEW Validation workflows: 11** ⭐
  - 9 language-specific validation workflows
  - 2 general validation workflows (pre-commit, CI/CD)

**Workflow Patterns:** 3 (ReAct, Plan-Execute, Reflection)

**Validation Coverage:**
- ✅ Full validation pipelines for 9 languages/frameworks
- ✅ Pre-commit hooks (<30s fast validation)
- ✅ CI/CD pipeline integration (comprehensive validation)
- ✅ Quality gates with coverage requirements
- ✅ Security scanning included

---

## See Also

- [Main Workflows](../) - General-purpose workflows
- [Templates](/templates/) - Configuration templates
- [Documentation](/docs/) - Comprehensive documentation
