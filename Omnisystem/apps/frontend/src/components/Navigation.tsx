import { Link, useNavigate } from 'react-router-dom'
import { useAuthStore } from '../stores/authStore'

export default function Navigation() {
  const navigate = useNavigate()
  const { user, logout } = useAuthStore()

  const handleLogout = () => {
    logout()
    navigate('/login')
  }

  return (
    <nav className="bg-blue-600 text-white shadow-lg">
      <div className="container mx-auto px-4 py-4 flex justify-between items-center">
        <div className="flex items-center gap-8">
          <Link to="/dashboard" className="text-2xl font-bold">
            PATHFINDER
          </Link>
          <div className="flex gap-4">
            <Link to="/dashboard" className="hover:bg-blue-700 px-3 py-2 rounded">
              Dashboard
            </Link>
            <Link to="/recommendations" className="hover:bg-blue-700 px-3 py-2 rounded">
              Recommended
            </Link>
            <Link to="/classrooms" className="hover:bg-blue-700 px-3 py-2 rounded">
              Classrooms
            </Link>
            <Link to="/progress" className="hover:bg-blue-700 px-3 py-2 rounded">
              Progress
            </Link>
            <Link to="/search" className="hover:bg-blue-700 px-3 py-2 rounded">
              Search
            </Link>
            <Link to="/admin" className="hover:bg-blue-700 px-3 py-2 rounded">
              Admin
            </Link>
          </div>
        </div>
        <div className="flex items-center gap-4">
          <span>{user?.name}</span>
          <button
            onClick={handleLogout}
            className="bg-red-600 hover:bg-red-700 px-4 py-2 rounded"
          >
            Logout
          </button>
        </div>
      </div>
    </nav>
  )
}
