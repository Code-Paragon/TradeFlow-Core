'use client';

import { useState, useEffect } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { submitInvoiceToSoroban, xlmToStroops, SOROBAN_CONFIG } from '@/utils/soroban';

// Zod schema for form validation
const invoiceSchema = z.object({
  debtorName: z.string()
    .min(2, 'Debtor name must be at least 2 characters')
    .max(100, 'Debtor name must be less than 100 characters')
    .trim(),
  invoiceAmount: z.string()
    .refine((val) => !isNaN(parseFloat(val)), 'Invalid amount format')
    .refine((val) => parseFloat(val) > 0, 'Amount must be greater than 0')
    .refine((val) => parseFloat(val) <= 1000000, 'Amount cannot exceed 1,000,000'),
  dueDate: z.string()
    .refine((val) => {
      const selectedDate = new Date(val);
      const today = new Date();
      today.setHours(0, 0, 0, 0);
      return selectedDate > today;
    }, 'Due date must be in the future'),
  documentUri: z.string()
    .url('Please enter a valid URL')
    .trim()
});

type InvoiceFormData = z.infer<typeof invoiceSchema>;

interface FeeCalculation {
  networkFee: number;
  protocolFee: number;
  totalFee: number;
  netAmount: number;
}

export default function InvoiceForm() {
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [feeCalculation, setFeeCalculation] = useState<FeeCalculation | null>(null);
  const [submitResult, setSubmitResult] = useState<{ success: boolean; message: string } | null>(null);

  // Initialize react-hook-form
  const {
    register,
    handleSubmit,
    watch,
    formState: { errors, isValid },
    reset,
    setValue,
    trigger
  } = useForm<InvoiceFormData>({
    resolver: zodResolver(invoiceSchema),
    mode: 'onChange',
    defaultValues: {
      debtorName: '',
      invoiceAmount: '',
      dueDate: '',
      documentUri: ''
    }
  });

  // Watch for changes in form values
  const watchedValues = watch();

  // Fee constants from config
  const NETWORK_FEE = SOROBAN_CONFIG.DEFAULT_NETWORK_FEE;
  const PROTOCOL_FEE_RATE = SOROBAN_CONFIG.DEFAULT_PROTOCOL_FEE_RATE;

  // Calculate fees whenever invoice amount changes and is valid
  useEffect(() => {
    if (watchedValues.invoiceAmount && !errors.invoiceAmount) {
      const amount = parseFloat(watchedValues.invoiceAmount);
      if (!isNaN(amount) && amount > 0) {
        const protocolFee = amount * PROTOCOL_FEE_RATE;
        const totalFee = NETWORK_FEE + protocolFee;
        const netAmount = amount - totalFee;

        setFeeCalculation({
          networkFee: NETWORK_FEE,
          protocolFee,
          totalFee,
          netAmount
        });
      } else {
        setFeeCalculation(null);
      }
    } else {
      setFeeCalculation(null);
    }
  }, [watchedValues.invoiceAmount, errors.invoiceAmount]);

  // Format payload for Soroban smart contract
  const formatSorobanPayload = (data: InvoiceFormData) => {
    return {
      debtor_name: data.debtorName,
      amount: xlmToStroops(parseFloat(data.invoiceAmount)), // Convert to stroops
      due_date: Math.floor(new Date(data.dueDate).getTime() / 1000), // Unix timestamp
      document_uri: data.documentUri,
      created_at: Math.floor(Date.now() / 1000), // Current timestamp
      metadata: {
        version: '1.0',
        source: 'tradeflow-web',
        network_fee: NETWORK_FEE,
        protocol_fee_rate: PROTOCOL_FEE_RATE
      }
    };
  };

  // Handle form submission
  const onSubmit = async (data: InvoiceFormData) => {
    setIsSubmitting(true);
    setSubmitResult(null);

    try {
      const sorobanPayload = formatSorobanPayload(data);
      
      console.log('Form submitted:', data);
      console.log('Soroban payload:', sorobanPayload);
      
      // TODO: Replace with actual contract address and user keys
      const contractAddress = SOROBAN_CONFIG.INVOICE_NFT_CONTRACT;
      const userPublicKey = 'G...'; // Get from user wallet
      const userSecretKey = 'S...'; // Get from user wallet (if needed)
      
      // Submit to Soroban smart contract
      const result = await submitInvoiceToSoroban(
        sorobanPayload,
        contractAddress,
        userPublicKey,
        userSecretKey
      );
      
      if (result.success) {
        setSubmitResult({
          success: true,
          message: `Invoice submitted successfully! Transaction: ${result.transaction_hash?.substring(0, 10)}... NFT ID: ${result.nft_id?.substring(0, 10)}...`
        });
        
        // Reset form after successful submission
        reset();
        setFeeCalculation(null);
      } else {
        setSubmitResult({
          success: false,
          message: `Error: ${result.error}`
        });
      }
      
    } catch (error) {
      console.error('Error submitting form:', error);
      setSubmitResult({
        success: false,
        message: 'Error submitting invoice. Please try again.'
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  const getMinDueDate = () => {
    const tomorrow = new Date();
    tomorrow.setDate(tomorrow.getDate() + 1);
    return tomorrow.toISOString().split('T')[0];
  };

  return (
    <div className="max-w-2xl mx-auto p-6 bg-white rounded-lg shadow-lg">
      <h2 className="text-2xl font-bold text-gray-900 mb-6">Submit Invoice for NFT Minting</h2>
      
      {/* Submission Result Alert */}
      {submitResult && (
        <div className={`mb-6 p-4 rounded-md ${
          submitResult.success 
            ? 'bg-green-50 border border-green-200 text-green-800' 
            : 'bg-red-50 border border-red-200 text-red-800'
        }`}>
          <p className="font-medium">{submitResult.message}</p>
        </div>
      )}
      
      <form onSubmit={handleSubmit(onSubmit)} className="space-y-6">
        {/* Debtor Name */}
        <div>
          <label htmlFor="debtorName" className="block text-sm font-medium text-gray-700 mb-2">
            Debtor Name *
          </label>
          <input
            type="text"
            id="debtorName"
            {...register('debtorName')}
            className={`w-full px-3 py-2 border rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 ${
              errors.debtorName ? 'border-red-500' : 'border-gray-300'
            }`}
            placeholder="Enter debtor company name"
            disabled={isSubmitting}
          />
          {errors.debtorName && (
            <p className="mt-1 text-sm text-red-600">{errors.debtorName.message}</p>
          )}
        </div>

        {/* Invoice Amount */}
        <div>
          <label htmlFor="invoiceAmount" className="block text-sm font-medium text-gray-700 mb-2">
            Invoice Amount (XLM) *
          </label>
          <input
            type="number"
            id="invoiceAmount"
            {...register('invoiceAmount')}
            step="0.0000001"
            min="0.0000001"
            max="1000000"
            className={`w-full px-3 py-2 border rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 ${
              errors.invoiceAmount ? 'border-red-500' : 'border-gray-300'
            }`}
            placeholder="0.0000000"
            disabled={isSubmitting}
          />
          {errors.invoiceAmount && (
            <p className="mt-1 text-sm text-red-600">{errors.invoiceAmount.message}</p>
          )}
        </div>

        {/* Due Date */}
        <div>
          <label htmlFor="dueDate" className="block text-sm font-medium text-gray-700 mb-2">
            Due Date *
          </label>
          <input
            type="date"
            id="dueDate"
            {...register('dueDate')}
            min={getMinDueDate()}
            className={`w-full px-3 py-2 border rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 ${
              errors.dueDate ? 'border-red-500' : 'border-gray-300'
            }`}
            disabled={isSubmitting}
          />
          {errors.dueDate && (
            <p className="mt-1 text-sm text-red-600">{errors.dueDate.message}</p>
          )}
        </div>

        {/* Document URI */}
        <div>
          <label htmlFor="documentUri" className="block text-sm font-medium text-gray-700 mb-2">
            Supporting Document URI *
          </label>
          <input
            type="url"
            id="documentUri"
            {...register('documentUri')}
            className={`w-full px-3 py-2 border rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 ${
              errors.documentUri ? 'border-red-500' : 'border-gray-300'
            }`}
            placeholder="https://example.com/invoice.pdf"
            disabled={isSubmitting}
          />
          {errors.documentUri && (
            <p className="mt-1 text-sm text-red-600">{errors.documentUri.message}</p>
          )}
          <p className="mt-1 text-sm text-gray-500">
            Provide a link to your invoice document (PDF, image, or other supporting documentation)
          </p>
        </div>

        {/* Fee Calculation */}
        {feeCalculation && (
          <div className="bg-blue-50 border border-blue-200 rounded-md p-4">
            <h3 className="text-lg font-semibold text-blue-900 mb-3">Fee Breakdown</h3>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-600">Invoice Amount:</span>
                <span className="font-medium">{parseFloat(watchedValues.invoiceAmount || '0').toFixed(7)} XLM</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Network Fee:</span>
                <span className="font-medium">{feeCalculation.networkFee.toFixed(7)} XLM</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Protocol Fee (0.5%):</span>
                <span className="font-medium">{feeCalculation.protocolFee.toFixed(7)} XLM</span>
              </div>
              <div className="flex justify-between pt-2 border-t border-blue-200">
                <span className="text-gray-600 font-semibold">Total Fees:</span>
                <span className="font-semibold text-blue-900">{feeCalculation.totalFee.toFixed(7)} XLM</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600 font-semibold">Net Amount:</span>
                <span className="font-semibold text-green-600">{feeCalculation.netAmount.toFixed(7)} XLM</span>
              </div>
            </div>
          </div>
        )}

        {/* Submit Button */}
        <div className="pt-4">
          <button
            type="submit"
            disabled={isSubmitting || !isValid}
            className="w-full bg-blue-600 text-white py-3 px-4 rounded-md font-medium hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors"
          >
            {isSubmitting ? 'Submitting...' : 'Submit Invoice for NFT Minting'}
          </button>
        </div>
      </form>
    </div>
  );
}
