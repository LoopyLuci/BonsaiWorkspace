// PATHFINDER Frontend - SignupPage
// Registration with COPPA compliance (parental consent for age < 13)

import React, { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { useDispatch } from 'react-redux';
import type { AppDispatch } from '../store';
import { authActions, uiActions } from '../store';
import apiClient from '../api-client';
import { AlertCircle, CheckCircle2, Mail, Lock, User } from 'lucide-react';

const SignupPage: React.FC = () => {
  const navigate = useNavigate();
  const dispatch = useDispatch<AppDispatch>();

  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [firstName, setFirstName] = useState('');
  const [lastName, setLastName] = useState('');
  const [age, setAge] = useState('');
  const [parentalConsent, setParentalConsent] = useState(false);
  const [termsAccepted, setTermsAccepted] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [showPassword, setShowPassword] = useState(false);

  // Validation
  const validateForm = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!email.trim()) {
      newErrors.email = 'Email is required';
    } else if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
      newErrors.email = 'Invalid email format';
    }

    if (!password.trim()) {
      newErrors.password = 'Password is required';
    } else if (password.length < 8) {
      newErrors.password = 'Password must be at least 8 characters';
    } else if (!/[A-Z]/.test(password)) {
      newErrors.password = 'Password must include uppercase letter';
    } else if (!/[0-9]/.test(password)) {
      newErrors.password = 'Password must include number';
    } else if (!/[!@#$%^&*]/.test(password)) {
      newErrors.password = 'Password must include special character (!@#$%^&*)';
    }

    if (!firstName.trim()) {
      newErrors.firstName = 'First name is required';
    }

    if (!lastName.trim()) {
      newErrors.lastName = 'Last name is required';
    }

    if (!age) {
      newErrors.age = 'Age is required';
    } else {
      const ageNum = parseInt(age);
      if (isNaN(ageNum) || ageNum < 5 || ageNum > 120) {
        newErrors.age = 'Age must be between 5 and 120';
      }
      if (ageNum < 13 && !parentalConsent) {
        newErrors.parentalConsent = 'Parental consent required for children under 13 (COPPA)';
      }
    }

    if (!termsAccepted) {
      newErrors.terms = 'You must accept terms of service';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!validateForm()) {
      dispatch(
        uiActions.showNotification({
          message: 'Please fix validation errors',
          type: 'error',
        })
      );
      return;
    }

    try {
      setIsLoading(true);

      // Register user
      const response = await apiClient.register({
        email,
        password,
        first_name: firstName,
        last_name: lastName,
        age: parseInt(age),
        parental_consent: parseInt(age) < 13 ? parentalConsent : false,
      });

      // Store token and user
      dispatch(authActions.setAuthToken(response.token));
      dispatch(
        authActions.setUser({
          id: response.user.id,
          email: response.user.email,
          first_name: response.user.first_name,
          last_name: response.user.last_name,
          age: response.user.age,
        })
      );

      dispatch(
        uiActions.showNotification({
          message: 'Welcome to PATHFINDER! 🎓',
          type: 'success',
        })
      );

      // Auto-login redirect
      navigate('/dashboard');
    } catch (error: any) {
      const message = error.response?.data?.message || 'Registration failed';
      dispatch(
        uiActions.showNotification({
          message,
          type: 'error',
        })
      );
    } finally {
      setIsLoading(false);
    }
  };

  const ageNum = age ? parseInt(age) : null;
  const requiresParentalConsent = ageNum && ageNum < 13;

  return (
    <div className="min-h-screen bg-gradient-to-br from-indigo-600 to-purple-600 flex items-center justify-center p-4">
      <div className="w-full max-w-md bg-white rounded-lg shadow-lg p-8">
        {/* HEADER */}
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-gray-900 mb-2">
            Join PATHFINDER 🎓
          </h1>
          <p className="text-gray-600">
            Learn languages faster with science
          </p>
        </div>

        {/* FORM */}
        <form onSubmit={handleSubmit} className="space-y-4">
          {/* NAME ROW */}
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                First Name
              </label>
              <input
                type="text"
                value={firstName}
                onChange={(e) => setFirstName(e.target.value)}
                className={`w-full px-3 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500 ${
                  errors.firstName ? 'border-red-500' : 'border-gray-300'
                }`}
                disabled={isLoading}
              />
              {errors.firstName && (
                <p className="text-red-500 text-xs mt-1">{errors.firstName}</p>
              )}
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Last Name
              </label>
              <input
                type="text"
                value={lastName}
                onChange={(e) => setLastName(e.target.value)}
                className={`w-full px-3 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500 ${
                  errors.lastName ? 'border-red-500' : 'border-gray-300'
                }`}
                disabled={isLoading}
              />
              {errors.lastName && (
                <p className="text-red-500 text-xs mt-1">{errors.lastName}</p>
              )}
            </div>
          </div>

          {/* EMAIL */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Email Address
            </label>
            <div className="relative">
              <Mail
                className="absolute left-3 top-3 text-gray-400"
                size={18}
              />
              <input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                className={`w-full pl-10 pr-3 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500 ${
                  errors.email ? 'border-red-500' : 'border-gray-300'
                }`}
                disabled={isLoading}
              />
            </div>
            {errors.email && (
              <p className="text-red-500 text-xs mt-1">{errors.email}</p>
            )}
          </div>

          {/* PASSWORD */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Password
            </label>
            <div className="relative">
              <Lock
                className="absolute left-3 top-3 text-gray-400"
                size={18}
              />
              <input
                type={showPassword ? 'text' : 'password'}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className={`w-full pl-10 pr-10 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500 ${
                  errors.password ? 'border-red-500' : 'border-gray-300'
                }`}
                disabled={isLoading}
              />
              <button
                type="button"
                onClick={() => setShowPassword(!showPassword)}
                className="absolute right-3 top-2 text-gray-400 hover:text-gray-600"
              >
                {showPassword ? '👁️' : '👁️‍🗨️'}
              </button>
            </div>
            {errors.password && (
              <p className="text-red-500 text-xs mt-1">{errors.password}</p>
            )}
            <p className="text-xs text-gray-500 mt-2">
              Min 8 chars, uppercase, number, special char
            </p>
          </div>

          {/* AGE */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Age
            </label>
            <input
              type="number"
              min="5"
              max="120"
              value={age}
              onChange={(e) => setAge(e.target.value)}
              className={`w-full px-3 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500 ${
                errors.age ? 'border-red-500' : 'border-gray-300'
              }`}
              disabled={isLoading}
            />
            {errors.age && (
              <p className="text-red-500 text-xs mt-1">{errors.age}</p>
            )}
          </div>

          {/* COPPA CONSENT */}
          {requiresParentalConsent && (
            <div className="bg-yellow-50 border border-yellow-200 rounded-lg p-3">
              <label className="flex items-start gap-2">
                <input
                  type="checkbox"
                  checked={parentalConsent}
                  onChange={(e) => setParentalConsent(e.target.checked)}
                  className="mt-1"
                  disabled={isLoading}
                />
                <span className="text-sm text-gray-700">
                  <strong>Parental Consent (COPPA):</strong> I confirm I am a parent/guardian
                  and give permission for this child to use PATHFINDER. We do not sell or share
                  personal data.
                </span>
              </label>
              {errors.parentalConsent && (
                <p className="text-red-500 text-xs mt-2">{errors.parentalConsent}</p>
              )}
            </div>
          )}

          {/* TERMS */}
          <label className="flex items-start gap-2">
            <input
              type="checkbox"
              checked={termsAccepted}
              onChange={(e) => setTermsAccepted(e.target.checked)}
              className="mt-1"
              disabled={isLoading}
            />
            <span className="text-sm text-gray-700">
              I accept the{' '}
              <a href="/terms" className="text-indigo-600 hover:underline">
                Terms of Service
              </a>{' '}
              and{' '}
              <a href="/privacy" className="text-indigo-600 hover:underline">
                Privacy Policy
              </a>
            </span>
          </label>
          {errors.terms && (
            <p className="text-red-500 text-xs">{errors.terms}</p>
          )}

          {/* SUBMIT */}
          <button
            type="submit"
            disabled={isLoading}
            className="w-full py-3 bg-indigo-600 hover:bg-indigo-700 disabled:bg-indigo-400 text-white font-semibold rounded-lg transition flex items-center justify-center gap-2"
          >
            {isLoading ? (
              <>
                <div className="animate-spin w-4 h-4 border-2 border-white border-t-transparent rounded-full"></div>
                Creating Account...
              </>
            ) : (
              <>
                <CheckCircle2 size={20} />
                Create Account
              </>
            )}
          </button>
        </form>

        {/* LOGIN LINK */}
        <p className="text-center text-gray-600 mt-6">
          Already have an account?{' '}
          <Link to="/login" className="text-indigo-600 hover:underline font-semibold">
            Log in
          </Link>
        </p>

        {/* PRIVACY NOTE */}
        <div className="bg-blue-50 border border-blue-200 rounded-lg p-3 mt-6">
          <p className="text-xs text-gray-700">
            <strong>Privacy First:</strong> PATHFINDER never sells data. We comply with GDPR,
            COPPA, and CCPA. Your learning data stays yours.
          </p>
        </div>
      </div>
    </div>
  );
};

export default SignupPage;
