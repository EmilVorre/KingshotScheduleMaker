import { useEffect, useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { api, Schedule } from '../api/client'

const DAYS = [
  { key: 'construction', name: 'Construction', icon: 'fa-hammer', color: 'orange' },
  { key: 'research', name: 'Research', icon: 'fa-flask', color: 'purple' },
  { key: 'troops', name: 'Troops', icon: 'fa-users', color: 'green' },
] as const

export default function ViewSchedulePage() {
  const { accountName, server } = useParams<{ accountName: string; server: string }>()
  const [day, setDay] = useState<(typeof DAYS)[number]['key']>('construction')
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [schedule, setSchedule] = useState<Schedule | null>(null)

  useEffect(() => {
    if (!accountName || !server) return
    setLoading(true)
    setError(null)
    api.getSchedule(accountName, parseInt(server, 10), day).then(({ ok, data, error: err }) => {
      if (ok && data) setSchedule(data)
      else setError(err || 'Failed to load schedule')
      setLoading(false)
    })
  }, [accountName, server, day])

  if (!accountName || !server) {
    return (
      <div className="container mx-auto px-4 py-8">
        <p className="text-red-400">Invalid URL</p>
        <Link to="/" className="text-blue-400 mt-4 inline-block">← Home</Link>
      </div>
    )
  }

  return (
    <div className="container mx-auto px-4 py-8 max-w-4xl">
      <header className="text-center mb-12">
        <h1 className="text-4xl font-bold text-blue-400 mb-4">
          <i className="fas fa-calendar-alt mr-3"></i>Schedule
        </h1>
        <p className="text-gray-400">{accountName} - Server {server}</p>
      </header>

      <div className="flex justify-center gap-4 mb-8 flex-wrap">
        {DAYS.map((d) => (
          <button
            key={d.key}
            onClick={() => setDay(d.key)}
            className={`px-6 py-3 rounded-lg font-semibold transition-all ${
              day === d.key
                ? d.color === 'orange'
                  ? 'bg-orange-600 text-white ring-2 ring-orange-400'
                  : d.color === 'purple'
                    ? 'bg-purple-600 text-white ring-2 ring-purple-400'
                    : 'bg-green-600 text-white ring-2 ring-green-400'
                : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
            }`}
          >
            <i className={`fas ${d.icon} mr-2`}></i>{d.name}
          </button>
        ))}
      </div>

      {loading && (
        <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
          <i className="fas fa-spinner fa-spin text-4xl text-blue-400 mb-4"></i>
          <p className="text-xl text-gray-400">Loading schedule...</p>
        </div>
      )}
      {error && (
        <div className="bg-red-900/50 border-l-4 border-red-500 text-red-200 p-4 rounded-lg">
          <i className="fas fa-exclamation-circle mr-2"></i>{error}
        </div>
      )}
      {!loading && !error && schedule?.appointments && (
        <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
          <h2 className="text-2xl font-bold text-white mb-6 text-center">{schedule.day_name}</h2>
          <div className="space-y-2">
            {schedule.appointments.map((slot, i) => (
              <div
                key={i}
                className={`flex items-center gap-4 py-2 border-b border-gray-700 last:border-0 ${slot.is_empty ? 'opacity-60' : ''}`}
              >
                <span className={`w-20 font-bold ${slot.is_empty ? 'text-gray-500' : 'text-blue-400'}`}>{slot.time}</span>
                <span className={slot.is_empty ? 'text-gray-500 italic' : 'text-white'}>{slot.is_empty ? '[EMPTY]' : (slot.player || '—')}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}
