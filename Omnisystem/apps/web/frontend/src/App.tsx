import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom'
import { QueryClientProvider, QueryClient } from '@tanstack/react-query'
import LoginPage from './pages/LoginPage'
import DashboardPage from './pages/DashboardPage'
import ExercisePage from './pages/ExercisePage'
import ClassroomPage from './pages/ClassroomPage'
import ProgressPage from './pages/ProgressPage'
import SearchPage from './pages/SearchPage'
import AdminDashboard from './pages/AdminDashboard'
import TeacherDashboard from './pages/TeacherDashboard'
import RecommendationsPage from './pages/RecommendationsPage'
import Navigation from './components/Navigation'
import { useAuthStore } from './stores/authStore'

const queryClient = new QueryClient()

export default function App() {
  const { token } = useAuthStore()

  return (
    <QueryClientProvider client={queryClient}>
      <Router>
        {token && <Navigation />}
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route
            path="/dashboard"
            element={token ? <DashboardPage /> : <Navigate to="/login" />}
          />
          <Route
            path="/exercises/:id"
            element={token ? <ExercisePage /> : <Navigate to="/login" />}
          />
          <Route
            path="/classrooms"
            element={token ? <ClassroomPage /> : <Navigate to="/login" />}
          />
          <Route
            path="/progress"
            element={token ? <ProgressPage /> : <Navigate to="/login" />}
          />
          <Route
            path="/search"
            element={token ? <SearchPage /> : <Navigate to="/login" />}
          />
          <Route
            path="/admin"
            element={token ? <AdminDashboard /> : <Navigate to="/login" />}
          />
          <Route
            path="/teacher"
            element={token ? <TeacherDashboard /> : <Navigate to="/login" />}
          />
          <Route
            path="/recommendations"
            element={token ? <RecommendationsPage /> : <Navigate to="/login" />}
          />
          <Route path="/" element={<Navigate to="/dashboard" />} />
        </Routes>
      </Router>
    </QueryClientProvider>
  )
}
