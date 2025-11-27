#!/bin/bash

# ABI Compatibility Verification Script
# Uses the closed source library as the source of truth

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLOSED_LIB="../closed_lib/libbambu_networking.dylib"
OPEN_LIB="target/release/libbambu_networking.dylib"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== ABI Compatibility Verification ===${NC}"
echo ""

# Check if libraries exist
if [ ! -f "$CLOSED_LIB" ]; then
    echo -e "${RED}ERROR: Closed source library not found at $CLOSED_LIB${NC}"
    exit 1
fi

if [ ! -f "$OPEN_LIB" ]; then
    echo -e "${RED}ERROR: Open source library not found at $OPEN_LIB${NC}"
    echo "Please build the library first with: cargo build --release"
    exit 1
fi

echo -e "${GREEN}✓ Found both libraries${NC}"
echo ""

# Extract symbols from closed source library (source of truth)
echo -e "${BLUE}Extracting symbols from closed source library...${NC}"
nm -gU "$CLOSED_LIB" | grep -E "^[0-9a-f]+ T _bambu" | awk '{print $3}' | sort > /tmp/closed_symbols.txt
CLOSED_COUNT=$(wc -l < /tmp/closed_symbols.txt | tr -d ' ')
echo -e "  Found ${GREEN}${CLOSED_COUNT}${NC} exported bambu_* symbols"

# Extract symbols from open source library
echo -e "${BLUE}Extracting symbols from open source library...${NC}"
nm -gU "$OPEN_LIB" | grep -E "^[0-9a-f]+ T _bambu" | awk '{print $3}' | sort > /tmp/open_symbols.txt
OPEN_COUNT=$(wc -l < /tmp/open_symbols.txt | tr -d ' ')
echo -e "  Found ${GREEN}${OPEN_COUNT}${NC} exported bambu_* symbols"
echo ""

# Find missing symbols (in closed but not in open)
echo -e "${BLUE}Checking for missing symbols...${NC}"
comm -23 /tmp/closed_symbols.txt /tmp/open_symbols.txt > /tmp/missing_symbols.txt
MISSING_COUNT=$(wc -l < /tmp/missing_symbols.txt | tr -d ' ')

if [ "$MISSING_COUNT" -gt 0 ]; then
    echo -e "${RED}✗ FAILED: ${MISSING_COUNT} symbols are missing from open source library:${NC}"
    cat /tmp/missing_symbols.txt | while read sym; do
        echo -e "  ${RED}✗${NC} $sym"
    done
    echo ""
else
    echo -e "${GREEN}✓ All required symbols are present${NC}"
    echo ""
fi

# Find extra symbols (in open but not in closed)
echo -e "${BLUE}Checking for extra symbols...${NC}"
comm -13 /tmp/closed_symbols.txt /tmp/open_symbols.txt > /tmp/extra_symbols.txt
EXTRA_COUNT=$(wc -l < /tmp/extra_symbols.txt | tr -d ' ')

if [ "$EXTRA_COUNT" -gt 0 ]; then
    echo -e "${YELLOW}⚠ WARNING: ${EXTRA_COUNT} extra symbols in open source library:${NC}"
    cat /tmp/extra_symbols.txt | while read sym; do
        echo -e "  ${YELLOW}+${NC} $sym"
    done
    echo ""
else
    echo -e "${GREEN}✓ No extra symbols${NC}"
    echo ""
fi

# Summary
echo -e "${BLUE}=== Summary ===${NC}"
echo -e "Closed source (reference): ${GREEN}${CLOSED_COUNT}${NC} symbols"
echo -e "Open source (yours):       ${GREEN}${OPEN_COUNT}${NC} symbols"
echo -e "Missing symbols:           ${RED}${MISSING_COUNT}${NC}"
echo -e "Extra symbols:             ${YELLOW}${EXTRA_COUNT}${NC}"
echo ""

# Run the C++ ABI test if it exists
if [ -f "$SCRIPT_DIR/test_abi" ]; then
    echo -e "${BLUE}Running detailed ABI tests...${NC}"
    "$SCRIPT_DIR/test_abi"
    echo ""
fi

# Final verdict
if [ "$MISSING_COUNT" -eq 0 ]; then
    echo -e "${GREEN}=== ABI VERIFICATION PASSED ===${NC}"
    echo -e "Your library exports all required symbols from the closed source library."
    if [ "$EXTRA_COUNT" -gt 0 ]; then
        echo -e "${YELLOW}Note: Your library has $EXTRA_COUNT extra symbols, which is usually okay.${NC}"
    fi
    exit 0
else
    echo -e "${RED}=== ABI VERIFICATION FAILED ===${NC}"
    echo -e "Your library is missing $MISSING_COUNT required symbols."
    echo -e "Please implement these functions to achieve ABI compatibility."
    exit 1
fi
