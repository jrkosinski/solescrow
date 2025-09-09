# Solescrow

A Solana escrow program built with Anchor.

## Setup

1. Install dependencies:
```bash
yarn install
```

2. Build the program:
```bash
anchor build
```

## Testing

### TypeScript Integration Tests
Run end-to-end tests:
```bash
anchor test
```

For clean validator state between runs:
```bash
./test-clean.sh
```

### Rust Unit Tests
Run fast isolated unit tests:
```bash
cargo test
```

## Development

- **Program code**: `programs/solescrow/src/`
- **TypeScript tests**: `tests/`
- **Test utilities**: `tests/utils.ts`