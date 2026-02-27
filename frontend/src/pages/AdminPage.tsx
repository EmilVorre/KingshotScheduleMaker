import { useEffect, useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { api } from '../api/client'

type Tab = 'upload' | 'config'

export default function AdminPage() {
  const { accountName, server } = useParams<{ accountName: string; server: string }>()
  const [isAuthenticated, setIsAuthenticated] = useState(false)
  const [password, setPassword] = useState('')
  const [loggingIn, setLoggingIn] = useState(false)
  const [loginError, setLoginError] = useState<string | null>(null)
  const [selectedFile, setSelectedFile] = useState<File | null>(null)
  const [uploading, setUploading] = useState(false)
  const [uploadStatus, setUploadStatus] = useState<{ type: 'success' | 'error'; message: string } | null>(null)
  const [activeTab, setActiveTab] = useState<Tab>('upload')
  const [config, setConfig] = useState({ alliances: [] as string[], include_non_of_above: true, kingdom_id: '140' })
  const [creatingForm, setCreatingForm] = useState(false)
  const [configStatus, setConfigStatus] = useState<{ type: 'success' | 'error'; message: string } | null>(null)
  const [createdFormUrl, setCreatedFormUrl] = useState<string | null>(null)
  const [createdFormCode, setCreatedFormCode] = useState<string | null>(null)
  const [currentForm, setCurrentForm] = useState<{ code: string; name: string; url: string } | null>(null)

  const serverNum = server ? parseInt(server, 10) : 0

  useEffect(() => {
    if (!accountName || !server) return
    loadConfig()
  }, [accountName, server])

  async function loadConfig() {
    const { ok, data } = await api.getPreviousFormConfig(accountName!, serverNum)
    if (ok && data?.config) {
      const c = data.config
      setConfig({
        alliances: (c.alliances || []).filter((a: string) => a !== 'Non of the above'),
        include_non_of_above: c.include_non_of_above !== false,
        kingdom_id: (c as { kingdom_id?: string }).kingdom_id || '140',
      })
    }
  }

  async function loadCurrentForm() {
    if (!accountName || !server) return
    const { ok, data } = await api.getCurrentForm(accountName, serverNum)
    if (ok && data) setCurrentForm(data)
  }

  async function handleLogin(e: React.FormEvent) {
    e.preventDefault()
    if (!accountName || !server || !password) return
    setLoggingIn(true)
    setLoginError(null)
    // Use main login to set session (needed for create form and getCurrentForm)
    const { ok, error } = await api.login(accountName, password)
    if (ok) {
      setIsAuthenticated(true)
      setPassword('')
      await loadConfig()
      loadCurrentForm()
    } else {
      setLoginError(error || 'Invalid password')
    }
    setLoggingIn(false)
  }

  function handleFileSelect(e: React.ChangeEvent<HTMLInputElement>) {
    setSelectedFile(e.target.files?.[0] || null)
    setUploadStatus(null)
  }

  async function handleUpload(e: React.FormEvent) {
    e.preventDefault()
    if (!accountName || !server || !selectedFile) {
      setUploadStatus({ type: 'error', message: 'Please select a file' })
      return
    }
    setUploading(true)
    setUploadStatus(null)
    const res = await api.uploadCsv(accountName, serverNum, selectedFile)
    if (res.ok && (res.data as { success?: boolean })?.success) {
      setUploadStatus({ type: 'success', message: (res.data as { message?: string })?.message || 'Schedule generated successfully!' })
      setSelectedFile(null)
    } else {
      setUploadStatus({ type: 'error', message: 'Error: ' + (res.error || 'Upload failed') })
    }
    setUploading(false)
  }

  async function handleCreateForm(e: React.FormEvent) {
    e.preventDefault()
    if (!accountName || !server) return
    const alliances = [...config.alliances.filter(Boolean)]
    if (config.include_non_of_above) alliances.push('Non of the above')
    if (alliances.length === 0) {
      setConfigStatus({ type: 'error', message: 'At least one alliance must be specified' })
      return
    }
    setCreatingForm(true)
    setConfigStatus(null)
    const { ok, data, error } = await api.createForm(accountName, serverNum, {
      alliances,
      include_non_of_above: config.include_non_of_above,
      kingdom_id: config.kingdom_id || '140',
      construction_times: { start_time: '00:00', end_time: undefined },
      research_times: { start_time: '00:00', end_time: undefined },
      troops_times: { start_time: '00:00', end_time: undefined },
      construction_truegold_mode: 'truegold_unlocked',
    })
    const formUrl = data?.url || (data as { form_url?: string })?.form_url
    if (ok && formUrl) {
      setCreatedFormUrl(formUrl)
      setCreatedFormCode((data?.code || (data as { form_code?: string })?.form_code) || null)
      setConfigStatus({ type: 'success', message: 'Form created successfully!' })
      loadCurrentForm()
    } else {
      setConfigStatus({ type: 'error', message: error || 'Failed to create form' })
    }
    setCreatingForm(false)
  }

  function copyFormUrl() {
    if (createdFormUrl) navigator.clipboard.writeText(createdFormUrl)
  }

  function copyCurrentFormUrl() {
    if (currentForm?.url) navigator.clipboard.writeText(currentForm.url)
  }

  if (!accountName || !server) {
    return (
      <div className="container mx-auto px-4 py-8">
        <p className="text-red-400">Invalid URL</p>
        <Link to="/" className="text-blue-400 mt-4 inline-block">← Home</Link>
      </div>
    )
  }

  return (
    <div className="container mx-auto px-4 py-8 max-w-2xl">
      <header className="text-center mb-12">
        <h1 className="text-4xl font-bold text-blue-400 mb-4">
          <i className="fas fa-upload mr-3"></i>CSV Upload
        </h1>
        <p className="text-gray-400">Upload CSV file to generate schedules (alternative to form submissions)</p>
      </header>

      {!isAuthenticated ? (
        <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
          <div className="text-center mb-8">
            <div className="inline-block bg-blue-900/50 rounded-full p-4 mb-4">
              <i className="fas fa-lock text-blue-400 text-3xl"></i>
            </div>
            <h2 className="text-3xl font-bold text-white mb-2">Login Required</h2>
            <p className="text-gray-400">Enter your password to access CSV upload</p>
          </div>
          <form onSubmit={handleLogin} className="space-y-6">
            <div>
              <label htmlFor="password" className="block text-sm font-semibold text-gray-300 mb-2">
                <i className="fas fa-key mr-2"></i>Password
              </label>
              <input
                type="password"
                id="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
                className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
              />
            </div>
            <button
              type="submit"
              disabled={loggingIn}
              className="w-full bg-blue-600 hover:bg-blue-700 text-white px-6 py-3 rounded-lg font-semibold transition-all disabled:opacity-50"
            >
              {loggingIn ? <i className="fas fa-spinner fa-spin mr-2"></i> : <i className="fas fa-sign-in-alt mr-2"></i>}
              {loggingIn ? 'Logging in...' : 'Login'}
            </button>
          </form>
          {loginError && (
            <div className="mt-4 p-4 bg-red-900/50 border-l-4 border-red-500 text-red-200 rounded">
              <i className="fas fa-exclamation-circle mr-2"></i>{loginError}
            </div>
          )}
        </div>
      ) : (
        <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
          {currentForm && (
            <div className="mb-8 p-6 bg-purple-900/50 border-l-4 border-purple-500 rounded-lg">
              <h3 className="text-xl font-bold text-purple-200 mb-3 flex items-center">
                <i className="fas fa-file-alt mr-2"></i>Current Form
              </h3>
              <div className="space-y-3">
                <div><p className="text-sm text-gray-300 mb-1">Form Name:</p><p className="text-lg font-semibold text-white">{currentForm.name}</p></div>
                <div><p className="text-sm text-gray-300 mb-1">Form Code:</p><p className="text-sm font-mono text-purple-300">{currentForm.code}</p></div>
                <div className="flex items-center gap-2 bg-gray-800 p-3 rounded-lg border border-gray-700">
                  <input type="text" value={currentForm.url} readOnly className="flex-1 bg-transparent text-white font-mono text-sm outline-none" />
                  <button onClick={copyCurrentFormUrl} className="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg transition-all">
                    <i className="fas fa-copy mr-2"></i>Copy Link
                  </button>
                </div>
                <div className="flex gap-2 flex-wrap">
                  <a href={currentForm.url} target="_blank" rel="noreferrer" className="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg transition-all text-sm">
                    <i className="fas fa-external-link-alt mr-2"></i>Open Form
                  </a>
                  <a href={`${currentForm.url}/stats`} target="_blank" rel="noreferrer" className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-all text-sm">
                    <i className="fas fa-chart-bar mr-2"></i>View Statistics
                  </a>
                </div>
              </div>
            </div>
          )}

          <div className="flex justify-center gap-4 mb-8 border-b border-gray-700">
            <button
              onClick={() => setActiveTab('upload')}
              className={`px-6 py-3 font-semibold transition-all border-b-2 ${activeTab === 'upload' ? 'text-blue-400 border-blue-400' : 'text-gray-400 border-transparent hover:text-gray-300'}`}
            >
              <i className="fas fa-upload mr-2"></i>Upload CSV
            </button>
            <button
              onClick={() => setActiveTab('config')}
              className={`px-6 py-3 font-semibold transition-all border-b-2 ${activeTab === 'config' ? 'text-blue-400 border-blue-400' : 'text-gray-400 border-transparent hover:text-gray-300'}`}
            >
              <i className="fas fa-cog mr-2"></i>Form Configuration
            </button>
          </div>

          {activeTab === 'upload' && (
            <>
              <div className="text-center mb-8">
                <h2 className="text-3xl font-bold text-white mb-2">Upload CSV File</h2>
                <p className="text-gray-400">Select a CSV file to process and generate schedules</p>
              </div>
              <form onSubmit={handleUpload} className="space-y-6">
                <div>
                  <label htmlFor="csv-file" className="block text-sm font-semibold text-gray-300 mb-2">
                    <i className="fas fa-file-csv mr-2"></i>Select CSV File
                  </label>
                  <input
                    type="file"
                    id="csv-file"
                    onChange={handleFileSelect}
                    accept=".csv"
                    required
                    className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white file:mr-4 file:py-2 file:px-4 file:rounded-lg file:border-0 file:bg-blue-600 file:text-white file:cursor-pointer"
                  />
                  {selectedFile && <p className="mt-2 text-sm text-gray-400"><i className="fas fa-file mr-2"></i>Selected: {selectedFile.name}</p>}
                </div>
                <button
                  type="submit"
                  disabled={uploading || !selectedFile}
                  className="w-full bg-green-600 hover:bg-green-700 text-white px-6 py-3 rounded-lg font-semibold transition-all disabled:opacity-50"
                >
                  {uploading ? <i className="fas fa-spinner fa-spin mr-2"></i> : <i className="fas fa-cloud-upload-alt mr-2"></i>}
                  {uploading ? 'Uploading and processing...' : 'Upload & Generate Schedule'}
                </button>
              </form>
              {uploadStatus && (
                <div className={`mt-4 p-4 rounded-lg ${uploadStatus.type === 'success' ? 'bg-green-900/50 border-l-4 border-green-500 text-green-200' : 'bg-red-900/50 border-l-4 border-red-500 text-red-200'}`}>
                  <i className={`fas fa-${uploadStatus.type === 'success' ? 'check-circle' : 'times-circle'} mr-2`}></i>
                  {uploadStatus.message}
                </div>
              )}
            </>
          )}

          {activeTab === 'config' && (
            <>
              <div className="text-center mb-8">
                <h2 className="text-3xl font-bold text-white mb-2">Create Form</h2>
                <p className="text-gray-400">Configure alliances and create a form to get a shareable link</p>
              </div>
              {createdFormUrl && (
                <div className="mb-8 p-6 bg-green-900/50 border-l-4 border-green-500 rounded-lg">
                  <h3 className="text-xl font-bold text-green-200 mb-3"><i className="fas fa-check-circle mr-2"></i>Form Created Successfully!</h3>
                  <div className="flex items-center gap-2 bg-gray-800 p-3 rounded-lg border border-gray-700 mb-3">
                    <input type="text" value={createdFormUrl} readOnly className="flex-1 bg-transparent text-white font-mono text-sm outline-none" />
                    <button onClick={copyFormUrl} className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-all">
                      <i className="fas fa-copy mr-2"></i>Copy
                    </button>
                  </div>
                  <div className="flex gap-2 flex-wrap">
                    <a href={createdFormUrl} target="_blank" rel="noreferrer" className="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg text-sm">
                      <i className="fas fa-external-link-alt mr-2"></i>Open Form
                    </a>
                    <a href={`${createdFormUrl}/stats`} target="_blank" rel="noreferrer" className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-sm">
                      <i className="fas fa-chart-bar mr-2"></i>View Statistics
                    </a>
                  </div>
                  {createdFormCode && <p className="text-sm text-gray-400 mt-3">Form Code: <span className="font-mono text-green-300">{createdFormCode}</span></p>}
                </div>
              )}
              <form onSubmit={handleCreateForm} className="space-y-8">
                <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
                  <h3 className="text-xl font-bold text-white mb-4"><i className="fas fa-crown mr-2"></i>Kingdom ID <span className="text-red-400">*</span></h3>
                  <p className="text-sm text-gray-400 mb-4">The kingdom ID used to validate applicants.</p>
                  <input
                    type="text"
                    value={config.kingdom_id}
                    onChange={(e) => setConfig((c) => ({ ...c, kingdom_id: e.target.value }))}
                    required
                    className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white mb-4"
                    placeholder="e.g. 140"
                  />
                </div>
                <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
                  <h3 className="text-xl font-bold text-white mb-4"><i className="fas fa-users mr-2"></i>Alliances</h3>
                  <p className="text-sm text-gray-400 mb-4">Add or remove alliance names. Players will select from this list.</p>
                  <button
                    type="button"
                    onClick={() => setConfig((c) => ({ ...c, include_non_of_above: !c.include_non_of_above }))}
                    className={`w-full px-4 py-3 rounded-lg font-medium transition-all flex items-center justify-center gap-2 mb-4 ${config.include_non_of_above ? 'bg-green-600/30 border-2 border-green-500 text-green-200' : 'bg-gray-700 border-2 border-gray-600 text-gray-400'}`}
                  >
                    <i className={`fas fa-${config.include_non_of_above ? 'check-circle' : 'times-circle'}`}></i>
                    {config.include_non_of_above ? 'Include "Non of the above"' : 'Exclude "Non of the above"'}
                  </button>
                  <div className="space-y-3">
                    {config.alliances.map((alliance, i) => (
                      <div key={i} className="flex items-center gap-2">
                        <input
                          value={alliance}
                          onChange={(e) => {
                            const next = [...config.alliances]
                            next[i] = e.target.value
                            setConfig((c) => ({ ...c, alliances: next }))
                          }}
                          className="flex-1 px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white"
                          placeholder="Alliance name"
                        />
                        <button
                          type="button"
                          onClick={() => setConfig((c) => ({ ...c, alliances: c.alliances.filter((_, j) => j !== i) }))}
                          className="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg"
                        >
                          <i className="fas fa-trash"></i>
                        </button>
                      </div>
                    ))}
                    <button
                      type="button"
                      onClick={() => setConfig((c) => ({ ...c, alliances: [...c.alliances, ''] }))}
                      className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg"
                    >
                      <i className="fas fa-plus mr-2"></i>Add Alliance
                    </button>
                  </div>
                </div>
                <p className="text-amber-400 text-sm">Note: Create Form requires main dashboard login. If you get an error, please login from the home page first.</p>
                <button
                  type="submit"
                  disabled={creatingForm}
                  className="w-full bg-purple-600 hover:bg-purple-700 text-white px-6 py-3 rounded-lg font-semibold transition-all disabled:opacity-50"
                >
                  {creatingForm ? <i className="fas fa-spinner fa-spin mr-2"></i> : <i className="fas fa-plus-circle mr-2"></i>}
                  {creatingForm ? 'Creating Form...' : 'Create Form'}
                </button>
              </form>
              {configStatus && (
                <div className={`mt-4 p-4 rounded-lg ${configStatus.type === 'success' ? 'bg-green-900/50 border-l-4 border-green-500 text-green-200' : 'bg-red-900/50 border-l-4 border-red-500 text-red-200'}`}>
                  <i className={`fas fa-${configStatus.type === 'success' ? 'check-circle' : 'times-circle'} mr-2`}></i>
                  {configStatus.message}
                </div>
              )}
            </>
          )}
        </div>
      )}

    </div>
  )
}
