'use client';

import { useState, useEffect } from 'react';
import InstallPrompt from '@/components/PWA/InstallPrompt';
import OfflineIndicator from '@/components/PWA/OfflineIndicator';
import SyncStatus from '@/components/PWA/SyncStatus';
import { useServiceWorker } from '@/hooks/useServiceWorker';
import { useOfflineSync } from '@/hooks/useOfflineSync';

import TokenSearchModal from '@/components/TokenSearchModal';

export default function HomePage() {
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);
  const [isTokenModalOpen, setIsTokenModalOpen] = useState(false);
  const { isReady, updateAvailable, applyUpdate } = useServiceWorker();
  const { isOnline, pendingItems } = useOfflineSync();

  useEffect(() => {
    // Register service worker and initialize PWA features
    if (isReady) {
      console.log('PWA is ready');
    }
  }, [isReady]);

  const navigationItems = [
    { name: 'Dashboard', href: '/', icon: '🏠' },
    { name: 'Submit Invoice', href: '/invoice', icon: '📝' },
    { name: 'Proofs', href: '/proofs', icon: '📄' },
    { name: 'Verify', href: '/verify', icon: '✅' },
    { name: 'Settings', href: '/settings', icon: '⚙️' },
  ];

  return (
    <div className="app-shell">
      {/* Update Available Banner */}
      {updateAvailable && (
        <div className="bg-blue-500 text-white px-4 py-2 text-center">
          <div className="max-w-7xl mx-auto flex items-center justify-between">
            <span className="text-sm">A new version is available!</span>
            <button
              onClick={applyUpdate}
              className="ml-4 px-3 py-1 bg-white text-blue-500 rounded text-sm font-medium hover:bg-blue-50 transition-colors"
            >
              Update Now
            </button>
          </div>
        </div>
      )}

      {/* Header */}
      <header className="app-header">
        <nav className="nav-container">
          <a href="/" className="nav-logo">
            <div className="w-8 h-8 bg-primary-500 rounded-lg flex items-center justify-center">
              <span className="text-white font-bold text-sm">V</span>
            </div>
            <span>Verinode</span>
          </a>

          {/* Desktop Navigation */}
          <div className="nav-menu">
            {navigationItems.map((item) => (
              <a
                key={item.name}
                href={item.href}
                className="flex items-center gap-2 px-3 py-2 rounded-md text-sm font-medium text-gray-700 hover:text-primary-600 hover:bg-gray-100 transition-colors"
              >
                <span>{item.icon}</span>
                <span>{item.name}</span>
              </a>
            ))}
          </div>

          {/* Mobile Menu Toggle */}
          <button
            onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
            className="nav-mobile-toggle"
            aria-label="Toggle navigation"
          >
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              {isMobileMenuOpen ? (
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              ) : (
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
              )}
            </svg>
          </button>
        </nav>
      </header>

      {/* Mobile Navigation Drawer */}
      <div className={`mobile-nav-overlay ${isMobileMenuOpen ? 'open' : ''}`} 
           onClick={() => setIsMobileMenuOpen(false)} />
      <nav className={`mobile-nav ${isMobileMenuOpen ? 'open' : ''}`}>
        <div className="p-4">
          <div className="flex items-center gap-2 mb-6">
            <div className="w-8 h-8 bg-primary-500 rounded-lg flex items-center justify-center">
              <span className="text-white font-bold text-sm">V</span>
            </div>
            <span className="font-semibold text-lg">Verinode</span>
          </div>
          
          <div className="space-y-2">
            {navigationItems.map((item) => (
              <a
                key={item.name}
                href={item.href}
                className="flex items-center gap-3 px-3 py-3 rounded-lg text-gray-700 hover:bg-gray-100 transition-colors"
                onClick={() => setIsMobileMenuOpen(false)}
              >
                <span className="text-xl">{item.icon}</span>
                <span className="font-medium">{item.name}</span>
              </a>
            ))}
          </div>
          
          <div className="mt-6 pt-6 border-t border-gray-200">
            <div className="space-y-3 text-sm">
              <div className="flex items-center justify-between">
                <span className="text-gray-600">Status:</span>
                <span className={`font-medium ${isOnline ? 'text-green-600' : 'text-red-600'}`}>
                  {isOnline ? 'Online' : 'Offline'}
                </span>
              </div>
              {pendingItems.length > 0 && (
                <div className="flex items-center justify-between">
                  <span className="text-gray-600">Pending:</span>
                  <span className="font-medium text-orange-600">{pendingItems.length} items</span>
                </div>
              )}
            </div>
          </div>
        </div>
      </nav>

      {/* Main Content */}
      <main className="app-main">
        <div className="content-container">
          {/* Hero Section */}
          <section className="text-center py-12">
            <h1 className="text-4xl font-bold text-gray-900 mb-4">
              Welcome to Verinode
            </h1>
            <p className="text-xl text-gray-600 mb-8 max-w-2xl mx-auto">
              Your Progressive Web App for TradeFlow verification and proof management. 
              Work offline, sync automatically, and get instant notifications.
            </p>
            
            <div className="flex flex-col sm:flex-row gap-4 justify-center">
              <a
                href="/invoice"
                className="btn btn-primary text-lg px-6 py-3"
              >
                Submit Invoice
              </a>
              <a
                href="/verify"
                className="btn btn-secondary text-lg px-6 py-3"
              >
                Start Verification
              </a>
              <a
                href="/proofs"
                className="btn btn-secondary text-lg px-6 py-3"
              >
                View Proofs
              </a>
              <button
                onClick={() => setIsTokenModalOpen(true)}
                className="btn btn-outline text-lg px-6 py-3"
              >
                Import Token
              </button>
            </div>
          </section>

          {isTokenModalOpen && <TokenSearchModal />}

          {/* Features Grid */}
          <section className="py-12">
            <h2 className="text-2xl font-bold text-center text-gray-900 mb-8">
              PWA Features
            </h2>
            
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
              {/* Offline Support */}
              <div className="card">
                <div className="w-12 h-12 bg-green-100 rounded-lg flex items-center justify-center mb-4">
                  <svg className="w-6 h-6 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                </div>
                <h3 className="text-lg font-semibold text-gray-900 mb-2">Offline Support</h3>
                <p className="text-gray-600">
                  Access your proofs and verification data even without an internet connection.
                </p>
              </div>

              {/* Push Notifications */}
              <div className="card">
                <div className="w-12 h-12 bg-blue-100 rounded-lg flex items-center justify-center mb-4">
                  <svg className="w-6 h-6 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" />
                  </svg>
                </div>
                <h3 className="text-lg font-semibold text-gray-900 mb-2">Push Notifications</h3>
                <p className="text-gray-600">
                  Get instant updates when verification status changes or new proofs are available.
                </p>
              </div>

              {/* Background Sync */}
              <div className="card">
                <div className="w-12 h-12 bg-purple-100 rounded-lg flex items-center justify-center mb-4">
                  <svg className="w-6 h-6 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                  </svg>
                </div>
                <h3 className="text-lg font-semibold text-gray-900 mb-2">Background Sync</h3>
                <p className="text-gray-600">
                  Automatically sync your data when you come back online, no manual intervention needed.
                </p>
              </div>

              {/* App Installation */}
              <div className="card">
                <div className="w-12 h-12 bg-orange-100 rounded-lg flex items-center justify-center mb-4">
                  <svg className="w-6 h-6 text-orange-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 18h.01M8 21h8a2 2 0 002-2V5a2 2 0 00-2-2H8a2 2 0 00-2 2v14a2 2 0 002 2z" />
                  </svg>
                </div>
                <h3 className="text-lg font-semibold text-gray-900 mb-2">Install as App</h3>
                <p className="text-gray-600">
                  Install Verinode on your device for a native app experience with quick access.
                </p>
              </div>

              {/* Fast Loading */}
              <div className="card">
                <div className="w-12 h-12 bg-yellow-100 rounded-lg flex items-center justify-center mb-4">
                  <svg className="w-6 h-6 text-yellow-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
                  </svg>
                </div>
                <h3 className="text-lg font-semibold text-gray-900 mb-2">Instant Loading</h3>
                <p className="text-gray-600">
                  App shell architecture ensures the app loads instantly even on slow connections.
                </p>
              </div>

              {/* Responsive Design */}
              <div className="card">
                <div className="w-12 h-12 bg-indigo-100 rounded-lg flex items-center justify-center mb-4">
                  <svg className="w-6 h-6 text-indigo-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
                  </svg>
                </div>
                <h3 className="text-lg font-semibold text-gray-900 mb-2">Responsive Design</h3>
                <p className="text-gray-600">
                  Optimized for all devices - mobile, tablet, and desktop with adaptive layouts.
                </p>
              </div>
            </div>
          </section>

          {/* Status Section */}
          <section className="py-12">
            <h2 className="text-2xl font-bold text-center text-gray-900 mb-8">
              Current Status
            </h2>
            
            <div className="max-w-2xl mx-auto">
              <div className="card">
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <span className="font-medium text-gray-700">Connection Status:</span>
                    <span className={`px-3 py-1 rounded-full text-sm font-medium ${
                      isOnline 
                        ? 'bg-green-100 text-green-800' 
                        : 'bg-red-100 text-red-800'
                    }`}>
                      {isOnline ? '🟢 Online' : '🔴 Offline'}
                    </span>
                  </div>
                  
                  <div className="flex items-center justify-between">
                    <span className="font-medium text-gray-700">PWA Status:</span>
                    <span className={`px-3 py-1 rounded-full text-sm font-medium ${
                      isReady 
                        ? 'bg-blue-100 text-blue-800' 
                        : 'bg-gray-100 text-gray-800'
                    }`}>
                      {isReady ? '✅ Ready' : '⏳ Loading...'}
                    </span>
                  </div>
                  
                  <div className="flex items-center justify-between">
                    <span className="font-medium text-gray-700">Pending Sync Items:</span>
                    <span className={`px-3 py-1 rounded-full text-sm font-medium ${
                      pendingItems.length > 0 
                        ? 'bg-orange-100 text-orange-800' 
                        : 'bg-green-100 text-green-800'
                    }`}>
                      {pendingItems.length} items
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </section>
        </div>
      </main>

      {/* Footer */}
      <footer className="app-footer">
        <p>&copy; 2024 Verinode - TradeFlow Verification. All rights reserved.</p>
      </footer>

      {/* PWA Components */}
      <InstallPrompt />
      <OfflineIndicator />
      <SyncStatus />
    </div>
  );
}
