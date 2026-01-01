#!/bin/bash

# GPU Swarm Test Runner Script
# This script runs comprehensive tests to achieve 100% coverage

set -e

echo "🚀 Starting GPU Swarm Test Suite"
echo "=================================="

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
COVERAGE_THRESHOLD=100
TEST_TIMEOUT=300 # 5 minutes
PARALLEL_JOBS=4

# Directories
CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPORTS_DIR="$CRATE_DIR/test-reports"
COVERAGE_DIR="$CRATE_DIR/coverage-reports"

# Create directories
mkdir -p "$REPORTS_DIR"
mkdir -p "$COVERAGE_DIR"

echo -e "${BLUE}📁 Test directories created${NC}"

# Function to print test results
print_result() {
    local test_name="$1"
    local result="$2"
    local color="$3"
    
    if [ "$result" = "PASS" ]; then
        echo -e "${color}✅ $test_name: $result${NC}"
    else
        echo -e "${color}❌ $test_name: $result${NC}"
    fi
}

# Function to run tests with timeout
run_test_with_timeout() {
    local test_cmd="$1"
    local test_name="$2"
    local timeout="$3"
    
    timeout "$timeout" bash -c "$test_cmd" > "$REPORTS_DIR/${test_name}.log" 2>&1
    local exit_code=$?
    
    if [ $exit_code -eq 0 ]; then
        print_result "$test_name" "PASS" "$GREEN"
        return 0
    elif [ $exit_code -eq 124 ]; then
        print_result "$test_name" "TIMEOUT" "$YELLOW"
        return 1
    else
        print_result "$test_name" "FAIL" "$RED"
        return 1
    fi
}

# Function to check coverage
check_coverage() {
    local coverage_file="$1"
    local threshold="$2"
    
    if [ ! -f "$coverage_file" ]; then
        echo -e "${RED}❌ Coverage file not found: $coverage_file${NC}"
        return 1
    fi
    
    # Extract coverage percentage (this will depend on the coverage tool output format)
    local coverage=$(grep -o '[0-9]*\.[0-9]*%' "$coverage_file" | head -1 | sed 's/%//')
    
    if [ -z "$coverage" ]; then
        echo -e "${YELLOW}⚠️  Could not extract coverage percentage${NC}"
        return 1
    fi
    
    echo "Coverage: $coverage%"
    
    if (( $(echo "$coverage >= $threshold" | bc -l) )); then
        echo -e "${GREEN}✅ Coverage threshold met: $coverage% >= $threshold%${NC}"
        return 0
    else
        echo -e "${RED}❌ Coverage threshold not met: $coverage% < $threshold%${NC}"
        return 1
    fi
}

echo -e "${BLUE}🧪 Running Unit Tests${NC}"
echo "------------------------"

# Run unit tests
echo "Running basic unit tests..."
run_test_with_timeout "cargo test --lib -- --nocapture" "unit_tests" "$TEST_TIMEOUT"

echo ""
echo -e "${BLUE}🧪 Running Integration Tests${NC}"
echo "------------------------------"

# Run integration tests
echo "Running comprehensive integration tests..."
run_test_with_timeout "cargo test --test integration_tests_comprehensive -- --nocapture" "integration_tests" "$TEST_TIMEOUT"

echo "Running unit tests with comprehensive coverage..."
run_test_with_timeout "cargo test --test unit_tests -- --nocapture" "comprehensive_unit_tests" "$TEST_TIMEOUT"

echo "Running warden tests..."
run_test_with_timeout "cargo test --test warden_tests -- --nocapture" "warden_tests" "$TEST_TIMEOUT"

echo "Running crown tests..."
run_test_with_timeout "cargo test --test crown_tests -- --nocapture" "crown_tests" "$TEST_TIMEOUT"

echo "Running test utils tests..."
run_test_with_timeout "cargo test --test test_utils -- --nocapture" "test_utils_tests" "$TEST_TIMEOUT"

echo ""
echo -e "${BLUE}🧪 Running Performance Tests${NC}"
echo "----------------------------"

# Run performance tests
echo "Running performance benchmarks..."
run_test_with_timeout "cargo test --test integration_tests_comprehensive -- --ignored --nocapture" "performance_tests" "$TEST_TIMEOUT"

echo ""
echo -e "${BLUE}🧪 Running Stress Tests${NC}"
echo "-----------------------"

# Run stress tests
echo "Running stress tests..."
run_test_with_timeout "cargo test --test integration_tests_comprehensive --test stress_tests -- --nocapture" "stress_tests" "$TEST_TIMEOUT"

echo ""
echo -e "${BLUE}🧪 Running Edge Case Tests${NC}"
echo "---------------------------"

# Run edge case tests
echo "Running edge case tests..."
run_test_with_timeout "cargo test --test integration_tests_comprehensive --test edge_case_tests -- --nocapture" "edge_case_tests" "$TEST_TIMEOUT"

echo ""
echo -e "${BLUE}🧪 Running Benchmark Tests${NC}"
echo "---------------------------"

# Run benchmark tests
echo "Running benchmark tests..."
run_test_with_timeout "cargo test --test test_utils --test benchmarks -- --nocapture" "benchmark_tests" "$TEST_TIMEOUT"

echo ""
echo -e "${BLUE}🧪 Generating Coverage Report${NC}"
echo "------------------------------"

# Generate coverage report using tarpaulin
echo "Generating coverage report with cargo-tarpaulin..."
if command -v cargo-tarpaulin &> /dev/null; then
    cargo tarpaulin --out Html --out Lcov --output-dir "$COVERAGE_DIR" --timeout 300 --verbose
    COVERAGE_RESULT=$?
    
    if [ $COVERAGE_RESULT -eq 0 ]; then
        echo -e "${GREEN}✅ Coverage report generated successfully${NC}"
        echo "Coverage report available at: $COVERAGE_DIR/tarpaulin-report.html"
        
        # Check if we achieved 100% coverage
        if [ -f "$COVERAGE_DIR/lcov.info" ]; then
            # Extract overall coverage
            OVERALL_COVERAGE=$(grep "LF:" "$COVERAGE_DIR/lcov.info" | awk -F: '{sum_lines+=$2} END {print sum_lines}')
            OVERALL_COVERED=$(grep "LH:" "$COVERAGE_DIR/lcov.info" | awk -F: '{sum_covered+=$2} END {print sum_covered}')
            
            if [ -n "$OVERALL_COVERAGE" ] && [ -n "$OVERALL_COVERED" ] && [ "$OVERALL_COVERAGE" -gt 0 ]; then
                COVERAGE_PERCENT=$(echo "scale=2; $OVERALL_COVERED * 100 / $OVERALL_COVERAGE" | bc)
                echo "Overall coverage: $COVERAGE_PERCENT%"
                
                if (( $(echo "$COVERAGE_PERCENT >= $COVERAGE_THRESHOLD" | bc -l) )); then
                    echo -e "${GREEN}🎉 100% COVERAGE ACHIEVED! 🎉${NC}"
                    echo -e "${GREEN}✅ All code paths are now tested${NC}"
                else
                    echo -e "${YELLOW}⚠️  Coverage below 100%: $COVERAGE_PERCENT%${NC}"
                    echo -e "${YELLOW}💡 Some code paths may need additional tests${NC}"
                fi
            fi
        fi
    else
        echo -e "${RED}❌ Coverage report generation failed${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  cargo-tarpaulin not found. Install with: cargo install cargo-tarpaulin${NC}"
    echo "Running basic test coverage check instead..."
    
    # Run tests with coverage using standard cargo test
    cargo test --coverage 2>/dev/null || echo "Coverage flag not supported in this Rust version"
fi

echo ""
echo -e "${BLUE}🧪 Running All Tests with Coverage${NC}"
echo "------------------------------------"

# Run all tests to ensure everything works together
echo "Running complete test suite..."
run_test_with_timeout "cargo test --all -- --nocapture" "complete_test_suite" "$TEST_TIMEOUT"

echo ""
echo -e "${BLUE}📊 Test Summary${NC}"
echo "=================="

# Count test results
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

for log_file in "$REPORTS_DIR"/*.log; do
    if [ -f "$log_file" ]; then
        test_name=$(basename "$log_file" .log)
        if grep -q "test result: ok" "$log_file"; then
            ((PASSED_TESTS++))
        elif grep -q "test result: FAILED" "$log_file"; then
            ((FAILED_TESTS++))
        fi
        ((TOTAL_TESTS++))
    fi
done

echo "Total tests run: $TOTAL_TESTS"
echo -e "Passed: ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed: ${RED}$FAILED_TESTS${NC}"

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "\n${GREEN}🎉 ALL TESTS PASSED! 🎉${NC}"
    echo -e "${GREEN}✅ GPU Swarm test suite completed successfully${NC}"
else
    echo -e "\n${RED}❌ Some tests failed. Check the logs in $REPORTS_DIR${NC}"
fi

echo ""
echo -e "${BLUE}📁 Test Reports${NC}"
echo "Reports directory: $REPORTS_DIR"
echo "Coverage directory: $COVERAGE_DIR"

echo ""
echo -e "${BLUE}🚀 Test Suite Complete${NC}"
echo "========================"
echo "Use 'cargo test' to run tests locally"
echo "Use 'cargo tarpaulin' to generate coverage reports"
echo "Use 'cargo bench' to run performance benchmarks"

# Exit with appropriate code
if [ $FAILED_TESTS -eq 0 ]; then
    exit 0
else
    exit 1
fi