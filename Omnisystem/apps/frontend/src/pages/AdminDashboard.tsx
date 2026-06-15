import { useQuery } from '@tanstack/react-query'
import axios from 'axios'

export default function AdminDashboard() {
  const { data: stats } = useQuery({
    queryKey: ['analytics'],
    queryFn: () =>
      axios.get('/api/v1/analytics/classroom/class_1/stats'),
  })

  return (
    <div className="min-h-screen bg-gray-50 p-8">
      <div className="container mx-auto">
        <h1 className="text-3xl font-bold mb-8">Admin Dashboard</h1>

        <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
          <div className="bg-white rounded-lg shadow p-6">
            <p className="text-gray-600 text-sm">Total Students</p>
            <p className="text-3xl font-bold text-blue-600">
              {stats?.data?.total_students || 0}
            </p>
          </div>

          <div className="bg-white rounded-lg shadow p-6">
            <p className="text-gray-600 text-sm">Avg Mastery</p>
            <p className="text-3xl font-bold text-green-600">
              {stats?.data?.average_mastery
                ? `${Math.round(stats.data.average_mastery * 100)}%`
                : '0%'
              }
            </p>
          </div>

          <div className="bg-white rounded-lg shadow p-6">
            <p className="text-gray-600 text-sm">Completion</p>
            <p className="text-3xl font-bold text-purple-600">
              {stats?.data?.completion_rate
                ? `${Math.round(stats.data.completion_rate * 100)}%`
                : '0%'
              }
            </p>
          </div>

          <div className="bg-white rounded-lg shadow p-6">
            <p className="text-gray-600 text-sm">At Risk</p>
            <p className="text-3xl font-bold text-red-600">
              {stats?.data?.at_risk_count || 0}
            </p>
          </div>
        </div>

        <div className="bg-white rounded-lg shadow p-6">
          <h2 className="text-xl font-bold mb-4">Quick Actions</h2>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <button className="p-4 bg-blue-100 hover:bg-blue-200 rounded-lg">
              Manage Teachers
            </button>
            <button className="p-4 bg-green-100 hover:bg-green-200 rounded-lg">
              View Reports
            </button>
            <button className="p-4 bg-purple-100 hover:bg-purple-200 rounded-lg">
              User Management
            </button>
            <button className="p-4 bg-yellow-100 hover:bg-yellow-200 rounded-lg">
              Settings
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
