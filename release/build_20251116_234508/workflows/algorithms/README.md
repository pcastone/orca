# Algorithm Workflows and Templates

Comprehensive workflows and templates for algorithm development, testing, optimization, and competitive programming.

## 📁 Directory Structure

```
workflows/algorithms/           # Algorithm workflows
├── develop.yaml               # Algorithm development from scratch
├── test.yaml                  # Comprehensive testing
├── benchmark.yaml             # Performance benchmarking
├── analyze_complexity.yaml    # Time/space complexity analysis
├── optimize.yaml              # Algorithm optimization
├── competitive_programming.yaml  # End-to-end CP workflow
├── validate.yaml              # Full validation pipeline
└── README.md                  # This file

templates/algorithms/          # Algorithm templates
├── searching/
│   └── binary_search.py      # Binary search variants
├── graph/
│   └── graph_algorithms.py   # BFS, DFS, Dijkstra, etc.
├── dynamic_programming/
│   └── dp_patterns.py        # Knapsack, LCS, LIS, etc.
├── sorting/
├── data_structures/
├── strings/
├── greedy/
└── backtracking/
```

---

## 🚀 Quick Start

### Develop New Algorithm

```bash
aco workflow execute algorithm_develop --input "Implement binary search"
```

### Test Algorithm

```bash
aco workflow execute algorithm_test --input "Test my sorting algorithm"
```

### Analyze Complexity

```bash
aco workflow execute algorithm_analyze_complexity --input "Analyze time complexity"
```

### Optimize Algorithm

```bash
aco workflow execute algorithm_optimize --input "Optimize from O(n²) to O(n)"
```

### Benchmark Performance

```bash
aco workflow execute algorithm_benchmark --input "Benchmark quicksort vs mergesort"
```

### Competitive Programming

```bash
aco workflow execute competitive_programming --input "Solve LeetCode problem 1"
```

### Full Validation

```bash
aco workflow execute algorithm_validate --input "Validate before submission"
```

---

## 📚 Workflows

### 1. Algorithm Development (`develop.yaml`)

**Purpose**: Implement algorithm from problem description

**Phases**:
1. Understand problem
2. Choose algorithm approach
3. Design algorithm (pseudocode)
4. Implement code
5. Test implementation
6. Optimize (if needed)

**Time**: 20-30 minutes

**Output**: Working algorithm with tests

**Example**:
```bash
aco workflow execute algorithm_develop --input "Implement merge sort"
```

---

### 2. Algorithm Testing (`test.yaml`)

**Purpose**: Comprehensive test coverage

**Test Categories**:
- Basic cases (normal inputs)
- Edge cases (boundaries)
- Corner cases (unusual inputs)
- Large inputs (stress testing)
- Random tests (correctness verification)

**Frameworks Supported**:
- Python: pytest
- C++: Google Test
- Java: JUnit

**Coverage Goals**:
- Line coverage: >90%
- Branch coverage: >80%
- Edge cases: 100%

**Example**:
```bash
aco workflow execute algorithm_test --input "Test binary search with edge cases"
```

---

### 3. Performance Benchmarking (`benchmark.yaml`)

**Purpose**: Measure algorithm performance

**Metrics**:
- Execution time
- Memory usage
- CPU cycles
- Scaling behavior

**Input Patterns**:
- Best case
- Average case (random)
- Worst case

**Sizes Tested**: 10, 100, 1K, 10K, 100K, 1M

**Output**: Performance report with complexity verification

**Example**:
```bash
aco workflow execute algorithm_benchmark --input "Benchmark sorting algorithms"
```

---

### 4. Complexity Analysis (`analyze_complexity.yaml`)

**Purpose**: Determine time and space complexity

**Analyzes**:
- Best case complexity
- Average case complexity
- Worst case complexity
- Space complexity
- Recurrence relations

**Methods**:
- Theoretical analysis
- Empirical verification
- Master Theorem application

**Example**:
```bash
aco workflow execute algorithm_analyze_complexity --input "What is the complexity?"
```

---

### 5. Algorithm Optimization (`optimize.yaml`)

**Purpose**: Improve algorithm performance

**Techniques**:
- Hash tables (O(n²) → O(n))
- Binary search (O(n) → O(log n))
- Dynamic Programming (O(2^n) → O(n²))
- Two pointers (O(n²) → O(n))
- Sliding window (O(n×k) → O(n))

**Reports**:
- Before/after comparison
- Speedup factor
- Trade-offs (time vs space)

**Example**:
```bash
aco workflow execute algorithm_optimize --input "Make this faster"
```

---

### 6. Competitive Programming (`competitive_programming.yaml`)

**Purpose**: End-to-end CP problem solving

**Workflow**:
1. Understand problem (2-3 min)
2. Plan solution (3-5 min)
3. Implement (10-15 min)
4. Test (3-5 min)
5. Debug (5-10 min)
6. Optimize (if time)

**Total Time**: 20-30 minutes per problem

**Platforms**: LeetCode, Codeforces, AtCoder, HackerRank

**Example**:
```bash
aco workflow execute competitive_programming --input "Solve LeetCode 1: Two Sum"
```

---

### 7. Full Validation (`validate.yaml`)

**Purpose**: Complete validation pipeline

**Phases**:
1. Correctness validation (all tests pass)
2. Complexity validation (meets requirements)
3. Edge case validation (handles all cases)
4. Stress testing (large/random inputs)
5. Performance validation (time/memory limits)

**Quality Gates**:
- ✅ All tests pass
- ✅ Complexity acceptable
- ✅ Edge cases handled
- ✅ Performance within limits

**Example**:
```bash
aco workflow execute algorithm_validate --input "Full validation before submission"
```

---

## 📖 Algorithm Templates

### Binary Search

**File**: `templates/algorithms/searching/binary_search.py`

**Variants**:
- Standard binary search
- Leftmost occurrence
- Rightmost occurrence

**Complexity**: O(log n)

### Graph Algorithms

**File**: `templates/algorithms/graph/graph_algorithms.py`

**Algorithms**:
- BFS: O(V + E)
- DFS: O(V + E)
- Dijkstra: O((V + E) log V)
- Bellman-Ford: O(VE)
- Floyd-Warshall: O(V³)
- Topological Sort: O(V + E)

### Dynamic Programming

**File**: `templates/algorithms/dynamic_programming/dp_patterns.py`

**Patterns**:
- 0/1 Knapsack: O(n × capacity)
- Longest Common Subsequence: O(m × n)
- Longest Increasing Subsequence: O(n log n)
- Coin Change: O(amount × n)
- Edit Distance: O(m × n)
- Matrix Chain Multiplication: O(n³)

---

## 🎯 Use Cases

### Interview Preparation

```bash
# Practice algorithm
aco workflow execute algorithm_develop --input "Implement two-pointer solution"

# Test thoroughly
aco workflow execute algorithm_test --input "Test with edge cases"

# Analyze complexity
aco workflow execute algorithm_analyze_complexity --input "Verify O(n) complexity"
```

### Competitive Programming

```bash
# Solve problem end-to-end
aco workflow execute competitive_programming --input "Solve Codeforces 1000A"

# Validate before submission
aco workflow execute algorithm_validate --input "Pre-submission check"
```

### Algorithm Research

```bash
# Develop algorithm
aco workflow execute algorithm_develop --input "Implement new sorting algorithm"

# Benchmark performance
aco workflow execute algorithm_benchmark --input "Compare with quicksort"

# Analyze complexity
aco workflow execute algorithm_analyze_complexity --input "Determine complexity class"
```

### Learning & Education

```bash
# Implement classic algorithm
aco workflow execute algorithm_develop --input "Implement Dijkstra's algorithm"

# Test understanding
aco workflow execute algorithm_test --input "Test with various graph types"

# Optimize
aco workflow execute algorithm_optimize --input "Improve using priority queue"
```

---

## 📊 Complexity Reference

### Time Complexities (from best to worst)

| Complexity | Name | Example Algorithms |
|------------|------|-------------------|
| O(1) | Constant | Array access, hash table lookup |
| O(log n) | Logarithmic | Binary search, balanced BST operations |
| O(n) | Linear | Linear search, single loop |
| O(n log n) | Linearithmic | Merge sort, heap sort, efficient sorts |
| O(n²) | Quadratic | Bubble sort, selection sort, nested loops |
| O(n³) | Cubic | Floyd-Warshall, naive matrix multiply |
| O(2^n) | Exponential | Recursive Fibonacci, subset generation |
| O(n!) | Factorial | Permutations, traveling salesman (brute force) |

### Constraint → Complexity Guide

| Constraint (n) | Max Complexity | Algorithms |
|----------------|----------------|------------|
| n ≤ 10 | O(n!) | Permutations, brute force |
| n ≤ 20 | O(2^n) | Backtracking, bitmask DP |
| n ≤ 500 | O(n³) | Floyd-Warshall, DP with 3 dimensions |
| n ≤ 5,000 | O(n²) | Nested loops, basic DP |
| n ≤ 100,000 | O(n log n) | Sorting, segment tree |
| n ≤ 1,000,000 | O(n) | Single pass, hash table |
| Any n | O(log n) | Binary search |
| Any n | O(1) | Mathematical formula |

---

## 🛠️ Tips & Best Practices

### Problem Solving

1. **Read carefully** - Understand constraints
2. **Start simple** - Brute force first, optimize later
3. **Test early** - Don't wait until code is complete
4. **Think edge cases** - Empty, single element, max size
5. **Verify complexity** - Check before implementing

### Coding

1. **Use templates** - Don't reinvent common patterns
2. **Name clearly** - Meaningful variable names
3. **Comment logic** - Explain non-obvious parts
4. **Handle errors** - Check inputs, edge cases
5. **Test thoroughly** - Multiple test cases

### Optimization

1. **Profile first** - Identify actual bottleneck
2. **Know trade-offs** - Time vs space
3. **Use right data structure** - Hash, heap, tree
4. **Avoid premature optimization** - Correct first, fast second
5. **Measure improvement** - Benchmark before/after

---

## 🏆 Common Patterns

### Two Pointers

**When to use**: Array/string problems, sorted data
**Complexity**: O(n)
**Example**: Two sum on sorted array, remove duplicates

### Sliding Window

**When to use**: Subarray/substring problems
**Complexity**: O(n)
**Example**: Maximum sum subarray, longest substring

### Hash Table

**When to use**: Need O(1) lookup
**Complexity**: O(n) time, O(n) space
**Example**: Two sum, frequency counting

### Binary Search

**When to use**: Sorted data, monotonic function
**Complexity**: O(log n)
**Example**: Search in sorted array, find peak element

### Dynamic Programming

**When to use**: Overlapping subproblems, optimal substructure
**Complexity**: Varies (often O(n²) or O(n³))
**Example**: Knapsack, LCS, shortest path with DP

### Greedy

**When to use**: Local optimum leads to global optimum
**Complexity**: Often O(n log n)
**Example**: Activity selection, Huffman coding

### Backtracking

**When to use**: Need all solutions, constraint satisfaction
**Complexity**: Exponential
**Example**: N-Queens, Sudoku, permutations

---

## 📦 Integration Examples

### Use as Pre-commit Hook

```bash
# .git/hooks/pre-commit
#!/bin/bash
aco workflow execute algorithm_validate --input "Validate algorithm changes"
```

### Use in CI/CD

```yaml
# .github/workflows/algorithms.yml
name: Algorithm Validation
on: [push, pull_request]
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run validation
        run: aco workflow execute algorithm_validate --input "CI validation"
```

### Batch Testing

```bash
# Test multiple algorithms
for algo in sorting/*.py; do
    aco workflow execute algorithm_test --input "Test $algo"
done
```

---

## 🎓 Learning Path

### Beginner

1. Start with `algorithm_develop` - Learn algorithm basics
2. Use templates - Study common patterns
3. Practice testing - Use `algorithm_test`

### Intermediate

1. Analyze complexity - Use `algorithm_analyze_complexity`
2. Optimize algorithms - Use `algorithm_optimize`
3. Solve CP problems - Use `competitive_programming`

### Advanced

1. Benchmark performance - Use `algorithm_benchmark`
2. Full validation - Use `algorithm_validate`
3. Create custom templates - Extend template library

---

## 📚 Resources

- **Algorithm Templates**: `/templates/algorithms/`
- **Example Problems**: LeetCode, Codeforces, AtCoder
- **Complexity Analysis**: Big-O Cheat Sheet
- **Data Structures**: Choose right tool for the job

---

## 🤝 Contributing

To add new algorithm templates:

1. Create template file in appropriate category
2. Include:
   - Algorithm description
   - Time/space complexity
   - Example usage
   - Test cases
3. Add to this README

---

## Summary

**Workflows**: 7 comprehensive workflows
**Templates**: 3+ categories with common algorithms
**Use Cases**: Interview prep, CP, research, learning
**Time Budget**: 20-30 minutes per problem
**Quality Gates**: Correctness, complexity, performance

Ready to solve algorithms efficiently! 🚀
