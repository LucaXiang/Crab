import React from 'react';
import { Routes, Route, Navigate } from 'react-router-dom';
import { ProtectedRoute } from '@/presentation/components/layout/ProtectedRoute';
import { ConsoleLayout } from '@/presentation/components/layout/ConsoleLayout';
import { StoreLayout } from '@/presentation/components/layout/StoreLayout';
import { LoginScreen } from '@/screens/Login/LoginScreen';
import { ForgotPasswordScreen } from '@/screens/ForgotPassword/ForgotPasswordScreen';
import { DashboardScreen } from '@/screens/Dashboard/DashboardScreen';
import { StoresScreen } from '@/screens/Stores/StoresScreen';
import { AuthScreen } from '@/screens/Auth/AuthScreen';
import { StoreOverviewScreen } from '@/screens/Store/Overview/StoreOverviewScreen';
import { LiveOrdersScreen } from '@/screens/Store/LiveOrders/LiveOrdersScreen';
import { OrdersScreen } from '@/screens/Store/Orders/OrdersScreen';
import { StatsScreen } from '@/screens/Store/Stats/StatsScreen';
import { StatsDetailScreen } from '@/screens/Store/Stats/StatsDetailScreen';
import { RedFlagsScreen } from '@/screens/Store/RedFlags/RedFlagsScreen';
import { SettingsScreen } from '@/screens/Settings/SettingsScreen';
import { AuditScreen } from '@/screens/Audit/AuditScreen';
import { StoreSettingsScreen } from '@/screens/Store/Settings/StoreSettingsScreen';
import { ProductManagement } from '@/features/product';
import { CategoryManagement } from '@/features/category';
import { TagManagement } from '@/features/tag';
import { AttributeManagement } from '@/features/attribute';
import { PriceRuleManagement } from '@/features/price-rule';
import { EmployeeManagement } from '@/features/employee';
import { ZoneManagement } from '@/features/zone';
import { TableManagement } from '@/features/table';
import { LabelTemplateManagement } from '@/features/label-template';

export const App: React.FC = () => (
  <Routes>
    <Route path="/login" element={<LoginScreen />} />
    <Route path="/forgot-password" element={<ForgotPasswordScreen />} />
    <Route path="/auth" element={<ProtectedRoute><AuthScreen /></ProtectedRoute>} />

    <Route element={<ProtectedRoute><ConsoleLayout /></ProtectedRoute>}>
      <Route path="/" element={<DashboardScreen />} />
      <Route path="/stores" element={<StoresScreen />} />
      <Route path="/audit" element={<AuditScreen />} />
      <Route path="/settings" element={<SettingsScreen />} />
    </Route>

    <Route path="/stores/:id" element={<ProtectedRoute><StoreLayout /></ProtectedRoute>}>
      <Route index element={<StoreSettingsScreen />} />
      <Route path="overview" element={<StoreOverviewScreen />} />
      <Route path="live" element={<LiveOrdersScreen />} />
      <Route path="orders" element={<OrdersScreen />} />
      <Route path="stats" element={<StatsScreen />} />
      <Route path="stats/:date" element={<StatsDetailScreen />} />
      <Route path="products" element={<ProductManagement />} />
      <Route path="categories" element={<CategoryManagement />} />
      <Route path="tags" element={<TagManagement />} />
      <Route path="attributes" element={<AttributeManagement />} />
      <Route path="price-rules" element={<PriceRuleManagement />} />
      <Route path="employees" element={<EmployeeManagement />} />
      <Route path="zones" element={<ZoneManagement />} />
      <Route path="tables" element={<TableManagement />} />
      <Route path="label-templates" element={<LabelTemplateManagement />} />
      <Route path="red-flags" element={<RedFlagsScreen />} />
    </Route>

    <Route path="*" element={<Navigate to="/" />} />
  </Routes>
);
