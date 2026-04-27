'use client';

import { useState } from 'react';
import InvoiceForm from '@/components/InvoiceForm';

export default function InvoicePage() {
  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <div className="bg-white shadow-sm border-b border-gray-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center">
              <h1 className="text-xl font-semibold text-gray-900">Invoice Submission</h1>
            </div>
            <div className="flex items-center space-x-4">
              <a
                href="/"
                className="text-gray-500 hover:text-gray-700 px-3 py-2 rounded-md text-sm font-medium"
              >
                Back to Dashboard
              </a>
            </div>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <main className="max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
        <div className="px-4 py-6 sm:px-0">
          <div className="mb-8">
            <div className="bg-blue-50 border border-blue-200 rounded-lg p-6">
              <h2 className="text-lg font-semibold text-blue-900 mb-2">
                Submit Your Invoice for NFT Minting
              </h2>
              <p className="text-blue-800">
                Fill out the form below to submit your real-world invoice for factoring. 
                Once submitted, your invoice will be minted as an NFT on the Stellar network, 
                enabling you to access immediate liquidity through our DeFi protocol.
              </p>
            </div>
          </div>

          <InvoiceForm />
        </div>
      </main>
    </div>
  );
}
