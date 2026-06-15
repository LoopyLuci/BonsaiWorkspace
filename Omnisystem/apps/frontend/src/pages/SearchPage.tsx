import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { searchAPI } from '../api'

export default function SearchPage() {
  const [query, setQuery] = useState('')
  const { data: results, isLoading } = useQuery({
    queryKey: ['search', query],
    queryFn: () => searchAPI.search(query),
    enabled: query.length > 0,
  })

  return (
    <div className="min-h-screen bg-gray-50 p-8">
      <div className="container mx-auto max-w-2xl">
        <h1 className="text-3xl font-bold mb-8">Search</h1>

        <div className="mb-8">
          <input
            type="text"
            placeholder="Search skills and exercises..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:border-blue-500"
          />
        </div>

        {isLoading && <div>Searching...</div>}

        {results?.data?.results && (
          <div className="space-y-4">
            <p className="text-gray-600">
              Found {results.data.results.length} results for "{query}"
            </p>
            {results.data.results.map((result: any) => (
              <div key={result.id} className="bg-white rounded-lg shadow p-6">
                <h3 className="text-lg font-bold">{result.title}</h3>
                <p className="text-gray-600 text-sm mt-1">Type: {result.result_type}</p>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
