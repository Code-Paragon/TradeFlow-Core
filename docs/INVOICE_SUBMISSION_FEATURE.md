# Invoice Submission Feature Documentation

## Overview

The invoice submission feature enables businesses to submit their real-world invoice details for NFT minting on the TradeFlow platform. This feature integrates with Soroban smart contracts on the Stellar network to tokenize invoices as NFTs, enabling immediate liquidity through DeFi protocols.

## Features Implemented

### 1. Comprehensive Form Validation
- **Client-side validation** using react-hook-form and zod schema
- **Real-time validation** with immediate feedback
- **Input sanitization** to prevent injection attacks
- **Field-specific validation rules**:
  - Debtor Name: 2-100 characters, required
  - Invoice Amount: > 0 XLM, ≤ 1,000,000 XLM, required
  - Due Date: Must be in the future, required
  - Document URI: Valid URL format, required

### 2. Fee Calculation and Display
- **Network Fee**: Fixed 0.001 XLM per transaction
- **Protocol Fee**: 0.5% of invoice amount
- **Real-time calculation** as user types
- **Transparent breakdown** showing all fees
- **Net amount calculation** after fees

### 3. Soroban Smart Contract Integration
- **Payload formatting** for Soroban contract compatibility
- **Amount conversion** to stroops (7 decimal places)
- **Timestamp formatting** for Unix timestamps
- **Metadata inclusion** for tracking and versioning
- **Error handling** for transaction failures

### 4. User Experience
- **Responsive design** for mobile and desktop
- **Loading states** during submission
- **Success/error feedback** with transaction details
- **Form reset** after successful submission
- **Accessibility features** with proper ARIA labels

## File Structure

```
frontend/src/
├── components/
│   └── InvoiceForm.tsx          # Main form component
├── app/
│   └── invoice/
│       └── page.tsx             # Invoice submission page
├── utils/
│   └── soroban.ts              # Soroban integration utilities
└── package.json                # Updated dependencies
```

## Dependencies

The following dependencies have been added to support the invoice submission feature:

```json
{
  "react-hook-form": "^7.48.2",
  "@hookform/resolvers": "^3.3.2",
  "zod": "^3.22.4",
  "stellar-sdk": "^12.0.0"
}
```

## API Integration

### Soroban Contract Interface

The form prepares data in the following format for the Soroban smart contract:

```typescript
interface SorobanInvoicePayload {
  debtor_name: string;
  amount: number;           // Amount in stroops (7 decimal places)
  due_date: number;         // Unix timestamp
  document_uri: string;
  created_at: number;       // Current timestamp
  metadata: {
    version: string;
    source: string;
    network_fee: number;
    protocol_fee_rate: number;
  };
}
```

### Fee Structure

- **Network Fee**: 0.001 XLM (10000 stroops)
- **Protocol Fee**: 0.5% of invoice amount
- **Total Fees**: Network Fee + Protocol Fee
- **Net Amount**: Invoice Amount - Total Fees

## Usage

### Accessing the Form

1. Navigate to the main dashboard
2. Click "Submit Invoice" in the navigation menu
3. Or access directly at `/invoice`

### Form Submission Process

1. **Fill in all required fields**:
   - Debtor Name: Company or individual name
   - Invoice Amount: Amount in XLM (7 decimal precision)
   - Due Date: Future date when payment is due
   - Document URI: URL to invoice documentation

2. **Review fee calculation**:
   - Network fee is automatically calculated
   - Protocol fee is 0.5% of invoice amount
   - Net amount shows what you'll receive

3. **Submit the form**:
   - Click "Submit Invoice for NFT Minting"
   - Wait for transaction processing
   - Receive confirmation with transaction hash and NFT ID

### Error Handling

The form provides clear error messages for:
- **Validation errors**: Invalid input formats or missing fields
- **Network errors**: Connection issues with Soroban
- **Transaction errors**: Smart contract execution failures
- **Insufficient balance**: Not enough XLM for fees

## Security Considerations

### Input Validation
- All inputs are sanitized before processing
- URL validation prevents malicious links
- Amount validation prevents overflow attacks
- Date validation ensures future dates only

### Smart Contract Security
- Payload is formatted according to contract specifications
- Amounts are converted to smallest units (stroops)
- Timestamps use Unix format for consistency
- Metadata includes version tracking

### Error Prevention
- Client-side validation reduces server load
- Real-time feedback improves user experience
- Graceful degradation for network issues
- Transaction status tracking for reliability

## Future Enhancements

### Planned Features
1. **Multi-step form** for complex invoices
2. **File upload** for direct document submission
3. **Batch submission** for multiple invoices
4. **Template system** for recurring invoices
5. **Advanced fee options** with priority transactions

### Integration Points
1. **Wallet connection** for automatic signing
2. **KYC verification** for compliance
3. **Credit scoring** integration
4. **Marketplace listing** for NFT trading
5. **Analytics dashboard** for tracking

## Testing

### Unit Tests
- Form validation logic
- Fee calculation accuracy
- Payload formatting
- Error handling

### Integration Tests
- Soroban contract interaction
- Transaction submission
- Error scenarios
- Network connectivity

### User Testing
- Form usability
- Error message clarity
- Mobile responsiveness
- Accessibility compliance

## Troubleshooting

### Common Issues

1. **Form validation fails**
   - Check all required fields are filled
   - Ensure valid URL format for document URI
   - Verify amount is within allowed range

2. **Transaction fails**
   - Check network connectivity
   - Verify sufficient XLM balance
   - Ensure Soroban contract is available

3. **Fee calculation incorrect**
   - Refresh the page to reset calculations
   - Check for JavaScript errors in console
   - Verify input format is correct

### Debug Information

Enable debug mode by checking browser console for:
- Form validation errors
- Soroban payload details
- Transaction responses
- Network request logs

## Support

For technical support or feature requests:
1. Check this documentation first
2. Review browser console for errors
3. Contact the development team
4. Submit issues through the project repository

---

*This documentation covers the invoice submission feature implementation as of version 1.0. For the latest updates, refer to the project repository.*
