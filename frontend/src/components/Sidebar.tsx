import { useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router-dom'
import { useAuth } from '../context/AuthContext'
import { api } from '../api/client'
import { ENABLE_GOOGLE_LOGIN, IS_DEV } from '../config'
import ProfileMenu from './ProfileMenu'

const PREP_TABS = [
  { key: 'schedule', label: 'Schedule', icon: 'fa-calendar-check', href: (d: string) => `/dashboard/${d}?tab=schedule` },
  { key: 'stats', label: 'Statistics', icon: 'fa-chart-bar', href: (d: string) => `/dashboard/${d}?tab=stats` },
  { key: 'create-form', label: 'Create Form', icon: 'fa-plus-circle', href: (d: string) => `/dashboard/${d}?tab=create-form` },
  { key: 'current-form', label: 'Current Form', icon: 'fa-file-alt', href: (d: string) => `/dashboard/${d}?tab=current-form` },
  { key: 'csv-operations', label: 'CSV Operations', icon: 'fa-file-csv', href: (d: string) => `/dashboard/${d}?tab=csv-operations` },
  { key: 'generate-schedule', label: 'Generate Schedule', icon: 'fa-calendar-alt', href: (d: string) => `/dashboard/${d}?tab=generate-schedule` },
  { key: 'info', label: 'How to Use', icon: 'fa-info-circle', href: () => '/info' },
] as const

export default function Sidebar() {
  const { accountName, serverNumber, playerId, inGameName, isAdmin, allianceAccess, serverOrgAccess, isValid, refresh } = useAuth()
  const location = useLocation()
  const navigate = useNavigate()
  const [showLogin, setShowLogin] = useState(false)
  const [showProfileMenu, setShowProfileMenu] = useState(false)
  const [collapsed, setCollapsed] = useState(false)
  const [prepExpanded, setPrepExpanded] = useState(true)
  const [allianceExpanded, setAllianceExpanded] = useState(true)
  const [serverOrgExpanded, setServerOrgExpanded] = useState(true)
  const [adminExpanded, setAdminExpanded] = useState(true)

  const isOnDashboard = location.pathname.startsWith('/dashboard/') && accountName
  const currentTab = new URLSearchParams(location.search).get('tab') || 'schedule'

  async function handleLogout() {
    await api.logout()
    await refresh()
    navigate('/')
    setShowProfileMenu(false)
  }

  const nameForDisplay = inGameName || accountName || ''
  const displayName = nameForDisplay ? nameForDisplay.charAt(0).toUpperCase() + nameForDisplay.slice(1) : ''
  const initial = displayName ? displayName.charAt(0) : '?'

  return (
    <aside className={`flex-shrink-0 min-h-screen h-screen bg-gray-900 border-r border-gray-700/50 flex flex-col overflow-hidden transition-all duration-300 ${collapsed ? 'w-20' : 'w-64'}`}>
      {/* Top row: User/Login + Collapse button */}
      <div className="border-b border-gray-700/50 flex-shrink-0 flex items-center gap-2 p-2">
        <div className={`min-w-0 ${collapsed ? 'flex-1 flex justify-center' : 'flex-1'}`}>
        {isValid && accountName ? (
          <ProfileMenu
            open={showProfileMenu}
            onClose={() => setShowProfileMenu(false)}
            accountName={accountName}
            onLogout={handleLogout}
            trigger={
              <button
                onClick={() => setShowProfileMenu((s) => !s)}
                className={`w-full flex items-center rounded-lg hover:bg-gray-800/50 transition-colors text-left ${
                  collapsed ? 'justify-center p-1' : 'gap-3 p-2'
                }`}
              >
                <div className={`rounded-full bg-purple-600 flex items-center justify-center text-white font-semibold flex-shrink-0 overflow-hidden relative ${
                  collapsed ? 'w-8 h-8 text-sm' : 'w-10 h-10 text-lg'
                }`}>
                  {playerId ? (
                    <>
                      <span className="absolute inset-0 flex items-center justify-center bg-purple-600">{initial}</span>
                      <img
                        key={playerId}
                        src={`/api/avatar/${playerId}`}
                        alt=""
                        className="absolute inset-0 w-full h-full object-cover"
                        onError={(e) => { e.currentTarget.style.display = 'none' }}
                      />
                    </>
                  ) : (
                    initial
                  )}
                </div>
                {!collapsed && (
                  <>
                    <div className="flex-1 min-w-0">
                      <p className="font-medium text-white truncate">{displayName}</p>
                      <p className="text-xs text-gray-400 truncate">
                        {serverNumber != null ? `Server ${serverNumber}` : 'Account'}
                      </p>
                    </div>
                    <i className={`fas fa-chevron-down text-gray-400 text-xs transition-transform ${showProfileMenu ? 'rotate-180' : ''}`}></i>
                  </>
                )}
              </button>
            }
          />
        ) : (
          <div>
            <button
              onClick={() => setShowLogin((s) => !s)}
              className={`bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-all flex items-center justify-center gap-2 ${
                collapsed ? 'p-2' : 'w-full px-4 py-2.5'
              }`}
              title="Login"
            >
              <i className="fas fa-sign-in-alt"></i>
              {!collapsed && <span>Login</span>}
            </button>
            {showLogin && !collapsed && (
              <div className="mt-3 space-y-2">
                {IS_DEV && (
                  <div className="flex gap-1 mb-2">
                    <a
                      href="/api/dev-login"
                      className="flex-1 flex items-center justify-center gap-1 px-2 py-2 bg-amber-600 hover:bg-amber-500 text-white rounded-lg text-sm font-medium"
                      title="Admin"
                    >
                      <i className="fas fa-bolt"></i>
                      Admin
                    </a>
                    <a
                      href="/api/dev-login-user"
                      className="flex-1 flex items-center justify-center gap-1 px-2 py-2 bg-amber-700 hover:bg-amber-600 text-white rounded-lg text-sm font-medium"
                      title="Normal user"
                    >
                      <i className="fas fa-user"></i>
                      User
                    </a>
                  </div>
                )}
                <div className="flex gap-2">
                  <a
                    href="/api/auth/discord"
                    className="flex-1 flex items-center justify-center gap-1 px-2 py-2 bg-[#5865F2] hover:bg-[#4752C4] text-white rounded-lg text-sm font-medium"
                  >
                    <i className="fab fa-discord"></i>
                    Discord
                  </a>
                  {ENABLE_GOOGLE_LOGIN && (
                    <a
                      href="/api/auth/google"
                      className="flex-1 flex items-center justify-center gap-1 px-2 py-2 bg-white hover:bg-gray-100 text-gray-700 rounded-lg text-sm font-medium border border-gray-300"
                    >
                      <i className="fab fa-google text-red-500"></i>
                      Google
                    </a>
                  )}
                </div>
              </div>
            )}
          </div>
        )}
        </div>
        <button
          onClick={() => setCollapsed((c) => !c)}
          className="flex-shrink-0 p-2 rounded-lg text-gray-400 hover:bg-gray-800/80 hover:text-white transition-all"
          title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        >
          <i className={`fas fa-chevron-left text-sm transition-transform duration-300 ${collapsed ? 'rotate-180' : ''}`}></i>
        </button>
      </div>

      {/* Navigation */}
      <nav className={`flex-1 overflow-y-auto py-3 ${collapsed ? 'px-0' : ''}`}>
        {!isValid && !collapsed && (
          <div className="mx-4 mb-3 px-3 py-2 bg-amber-900/50 border border-amber-600/50 rounded-lg text-amber-200 text-sm">
            <i className="fas fa-lock mr-2"></i>
            Log in to use this function
          </div>
        )}

        {/* Admin Resources - only when admin */}
        {isAdmin && !collapsed && (
          <div className="mt-4 px-4 mb-4">
            <button
              onClick={() => setAdminExpanded((e) => !e)}
              className="flex items-center gap-2 w-full text-left text-xs font-medium text-gray-500 uppercase tracking-wider mb-2 hover:text-gray-400 transition-colors py-1 rounded"
            >
              <i className={`fas fa-chevron-down text-xs transition-transform duration-200 ${adminExpanded ? '' : '-rotate-90'}`}></i>
              Admin Resources
            </button>
            {adminExpanded && (
              <div className="mt-0.5 space-y-0.5">
                <Link
                  to="/admin-resources"
                  className={`flex items-center justify-between gap-2 px-3 py-2 rounded-lg text-sm transition-all ${
                    location.pathname === '/admin-resources'
                      ? 'bg-amber-600/90 text-white'
                      : 'text-gray-400 hover:bg-gray-800/80 hover:text-white'
                  }`}
                >
                  <span className="flex items-center gap-2">
                    <i className="fas fa-shield-alt w-4"></i>
                    Manage Admins
                  </span>
                  <i className="fas fa-chevron-right text-xs opacity-60"></i>
                </Link>
                <Link
                  to="/admin-feedback"
                  className={`flex items-center justify-between gap-2 px-3 py-2 rounded-lg text-sm transition-all ${
                    location.pathname === '/admin-feedback'
                      ? 'bg-amber-600/90 text-white'
                      : 'text-gray-400 hover:bg-gray-800/80 hover:text-white'
                  }`}
                >
                  <span className="flex items-center gap-2">
                    <i className="fas fa-comments w-4"></i>
                    Feedback
                  </span>
                  <i className="fas fa-chevron-right text-xs opacity-60"></i>
                </Link>
                <Link
                  to="/admin-alliance-applications"
                  className={`flex items-center justify-between gap-2 px-3 py-2 rounded-lg text-sm transition-all ${
                    location.pathname === '/admin-alliance-applications'
                      ? 'bg-amber-600/90 text-white'
                      : 'text-gray-400 hover:bg-gray-800/80 hover:text-white'
                  }`}
                >
                  <span className="flex items-center gap-2">
                    <i className="fas fa-file-signature w-4"></i>
                    Alliance Applications
                  </span>
                  <i className="fas fa-chevron-right text-xs opacity-60"></i>
                </Link>
              </div>
            )}
          </div>
        )}

        {/* Alliance Organisation - same level as Prep */}
        {!collapsed && (
          <div className="mt-4 px-4 mb-4">
            <button
              onClick={() => setAllianceExpanded((e) => !e)}
              className="flex items-center gap-2 w-full text-left text-xs font-medium text-gray-500 uppercase tracking-wider mb-2 hover:text-gray-400 transition-colors py-1 rounded"
            >
              <i className={`fas fa-chevron-down text-xs transition-transform duration-200 ${allianceExpanded ? '' : '-rotate-90'}`}></i>
              Alliance Organisation
            </button>
            {allianceExpanded && (
              <div className="mt-0.5 space-y-0.5">
                {!allianceAccess && (
                  <Link
                    to={accountName ? `/dashboard/${accountName}?tab=alliance-application` : '/'}
                    className={`flex items-center justify-between gap-2 px-3 py-2 rounded-lg text-sm transition-all ${
                      isOnDashboard && currentTab === 'alliance-application'
                        ? 'bg-indigo-600/90 text-white'
                        : 'text-gray-400 hover:bg-gray-800/80 hover:text-white'
                    }`}
                  >
                    <span className="flex items-center gap-2">
                      <i className="fas fa-file-signature w-4"></i>
                      Alliance Application
                    </span>
                    <i className="fas fa-chevron-right text-xs opacity-60"></i>
                  </Link>
                )}
                {[
                  { key: 'alliance-organisation', label: 'Alliance Organisation', icon: 'fa-sitemap' },
                  { key: 'giftcode-automation', label: 'Giftcode Automation', icon: 'fa-gift' },
                  { key: 'swordland', label: 'Swordland', icon: 'fa-landmark' },
                  { key: 'tri-alliance', label: 'Tri Alliance', icon: 'fa-users-cog' },
                ].map((tab) => {
                  const canUse = allianceAccess && isValid && accountName
                  const isActive = isOnDashboard && currentTab === tab.key
                  const tabContent = (
                    <span className="flex items-center justify-between gap-2 flex-1">
                      <span className="flex items-center gap-2">
                        <i className={`fas ${tab.icon} w-4`}></i>
                        {tab.label}
                      </span>
                      <i className="fas fa-chevron-right text-xs opacity-60"></i>
                    </span>
                  )
                  return canUse ? (
                    <Link
                      key={tab.key}
                      to={accountName ? `/dashboard/${accountName}?tab=${tab.key}` : '/'}
                      className={`flex items-center justify-between gap-2 px-3 py-2 rounded-lg text-sm transition-all ${
                        isActive
                          ? 'bg-indigo-600/90 text-white'
                          : 'text-gray-400 hover:bg-gray-800/80 hover:text-white'
                      }`}
                    >
                      {tabContent}
                    </Link>
                  ) : (
                    <span
                      key={tab.key}
                      className="flex items-center px-3 py-2 rounded-lg text-sm text-gray-500 cursor-not-allowed opacity-70"
                      title="Requires approved alliance access"
                    >
                      {tabContent}
                    </span>
                  )
                })}
              </div>
            )}
          </div>
        )}

        {/* Server Organisation */}
        {!collapsed && (
          <div className="mt-4 px-4 mb-4">
            <button
              onClick={() => setServerOrgExpanded((e) => !e)}
              className="flex items-center gap-2 w-full text-left text-xs font-medium text-gray-500 uppercase tracking-wider mb-2 hover:text-gray-400 transition-colors py-1 rounded"
            >
              <i className={`fas fa-chevron-down text-xs transition-transform duration-200 ${serverOrgExpanded ? '' : '-rotate-90'}`}></i>
              Server Organisation
            </button>
            {serverOrgExpanded && (
              <div className="mt-0.5 space-y-0.5">
                {[
                  { key: 'manage-server-org', label: 'Manage server', icon: 'fa-server' },
                  { key: 'tyrant', label: 'Tyrant', icon: 'fa-dragon' },
                ].map((tab) => {
                  const canUse =
                    isValid &&
                    accountName &&
                    (tab.key === 'manage-server-org' || serverOrgAccess)
                  const isActive = isOnDashboard && currentTab === tab.key
                  const tabContent = (
                    <span className="flex items-center justify-between gap-2 flex-1">
                      <span className="flex items-center gap-2">
                        <i className={`fas ${tab.icon} w-4`}></i>
                        {tab.label}
                      </span>
                      <i className="fas fa-chevron-right text-xs opacity-60"></i>
                    </span>
                  )
                  return canUse ? (
                    <Link
                      key={tab.key}
                      to={accountName ? `/dashboard/${accountName}?tab=${tab.key}` : '/'}
                      className={`flex items-center justify-between gap-2 px-3 py-2 rounded-lg text-sm transition-all ${
                        isActive ? 'bg-teal-700/90 text-white' : 'text-gray-400 hover:bg-gray-800/80 hover:text-white'
                      }`}
                    >
                      {tabContent}
                    </Link>
                  ) : (
                    <span
                      key={tab.key}
                      className="flex items-center px-3 py-2 rounded-lg text-sm text-gray-500 cursor-not-allowed opacity-70"
                      title="Join a server workspace first (invite) or create one here after access"
                    >
                      {tabContent}
                    </span>
                  )
                })}
              </div>
            )}
          </div>
        )}

        {isAdmin && collapsed && (
          <div className="mt-2 space-y-0.5">
            <Link
              to="/admin-resources"
              className={`flex justify-center py-2 rounded-lg mx-2 transition-all ${
                location.pathname === '/admin-resources'
                  ? 'bg-amber-600/90 text-white'
                  : 'text-gray-400 hover:bg-gray-800/80 hover:text-white'
              }`}
              title="Manage Admins"
            >
              <i className="fas fa-shield-alt w-5"></i>
            </Link>
            <Link
              to="/admin-feedback"
              className={`flex justify-center py-2 rounded-lg mx-2 transition-all ${
                location.pathname === '/admin-feedback'
                  ? 'bg-amber-600/90 text-white'
                  : 'text-gray-400 hover:bg-gray-800/80 hover:text-white'
              }`}
              title="Feedback"
            >
              <i className="fas fa-comments w-5"></i>
            </Link>
          </div>
        )}

        {/* Prep section - always visible */}
        {!collapsed && (
          <div className="mt-4 px-4">
            <button
              onClick={() => setPrepExpanded((e) => !e)}
              className="flex items-center gap-2 w-full text-left text-xs font-medium text-gray-500 uppercase tracking-wider mb-2 hover:text-gray-400 transition-colors py-1 rounded"
            >
              <i className={`fas fa-chevron-down text-xs transition-transform duration-200 ${prepExpanded ? '' : '-rotate-90'}`}></i>
              Prep
            </button>
            {prepExpanded && (
              <div className="mt-0.5">
                {PREP_TABS.map((tab) => {
                  const href = tab.key === 'info' ? '/info' : (accountName ? tab.href(accountName) : '/')
                  const isActive = tab.key === 'info'
                    ? location.pathname === '/info'
                    : isOnDashboard && currentTab === tab.key
                  const canUse = tab.key === 'info' || (isValid && accountName)
                  const tabContent = (
                    <span className="flex items-center justify-between gap-2 flex-1">
                      <span className="flex items-center gap-2">
                        <i className={`fas ${tab.icon} w-4`}></i>
                        {tab.label}
                      </span>
                      <i className="fas fa-chevron-right text-xs opacity-60"></i>
                    </span>
                  )
                  return canUse ? (
                    <Link
                      key={tab.key}
                      to={href}
                      className={`flex items-center px-3 py-2 rounded-lg text-sm transition-all ${
                        isActive
                          ? 'bg-blue-600/90 text-white'
                          : 'text-gray-400 hover:bg-gray-800/80 hover:text-white'
                      }`}
                    >
                      {tabContent}
                    </Link>
                  ) : (
                    <span
                      key={tab.key}
                      className="flex items-center px-3 py-2 rounded-lg text-sm text-gray-500 cursor-not-allowed opacity-70"
                    >
                      {tabContent}
                    </span>
                  )
                })}
              </div>
            )}
          </div>
        )}

        {/* When collapsed, show icon-only links */}
        {collapsed && (
          <div className="mt-2 space-y-0.5">
            {isValid && accountName && (
              <>
                {!allianceAccess && (
                  <Link
                    to={`/dashboard/${accountName}?tab=alliance-application`}
                    className={`flex justify-center py-2 rounded-lg mx-2 transition-all ${
                      isOnDashboard && currentTab === 'alliance-application'
                        ? 'bg-indigo-600/90 text-white'
                        : 'text-gray-400 hover:bg-gray-800/80 hover:text-white'
                    }`}
                    title="Alliance Application"
                  >
                    <i className="fas fa-file-signature w-5"></i>
                  </Link>
                )}
                {[
                  { key: 'alliance-organisation', label: 'Alliance Organisation', icon: 'fa-sitemap' },
                  { key: 'giftcode-automation', label: 'Giftcode Automation', icon: 'fa-gift' },
                  { key: 'swordland', label: 'Swordland', icon: 'fa-landmark' },
                  { key: 'tri-alliance', label: 'Tri Alliance', icon: 'fa-users-cog' },
                ].map((tab) => {
                  const canUse = allianceAccess
                  const isActive = isOnDashboard && currentTab === tab.key
                  return canUse ? (
                    <Link
                      key={tab.key}
                      to={`/dashboard/${accountName}?tab=${tab.key}`}
                      className={`flex justify-center py-2 rounded-lg mx-2 transition-all ${
                        isActive
                          ? 'bg-indigo-600/90 text-white'
                          : 'text-gray-400 hover:bg-gray-800/80 hover:text-white'
                      }`}
                      title={tab.label}
                    >
                      <i className={`fas ${tab.icon} w-5`}></i>
                    </Link>
                  ) : (
                    <span
                      key={tab.key}
                      className="flex justify-center py-2 rounded-lg mx-2 text-gray-500 cursor-not-allowed opacity-70"
                      title={`${tab.label} (requires approved alliance access)`}
                    >
                      <i className={`fas ${tab.icon} w-5`}></i>
                    </span>
                  )
                })}
                {[
                  { key: 'manage-server-org', label: 'Manage server', icon: 'fa-server' },
                  { key: 'tyrant', label: 'Tyrant', icon: 'fa-dragon' },
                ].map((tab) => {
                  const canUse = tab.key === 'manage-server-org' || serverOrgAccess
                  const isActive = isOnDashboard && currentTab === tab.key
                  return canUse ? (
                    <Link
                      key={tab.key}
                      to={`/dashboard/${accountName}?tab=${tab.key}`}
                      className={`flex justify-center py-2 rounded-lg mx-2 transition-all ${
                        isActive
                          ? 'bg-teal-700/90 text-white'
                          : 'text-gray-400 hover:bg-gray-800/80 hover:text-white'
                      }`}
                      title={tab.label}
                    >
                      <i className={`fas ${tab.icon} w-5`}></i>
                    </Link>
                  ) : (
                    <span
                      key={tab.key}
                      className="flex justify-center py-2 rounded-lg mx-2 text-gray-500 cursor-not-allowed opacity-70"
                      title={`${tab.label} — create or join a server workspace first`}
                    >
                      <i className={`fas ${tab.icon} w-5`}></i>
                    </span>
                  )
                })}
              </>
            )}
            {PREP_TABS.map((tab) => {
              const href = tab.key === 'info' ? '/info' : (accountName ? tab.href(accountName) : '/')
              const isActive = tab.key === 'info'
                ? location.pathname === '/info'
                : isOnDashboard && currentTab === tab.key
              const canUse = isValid && (tab.key === 'info' || accountName)
              return canUse ? (
                <Link
                  key={tab.key}
                  to={href}
                  className={`flex justify-center py-2 rounded-lg mx-2 transition-all ${
                    isActive ? 'bg-blue-600/90 text-white' : 'text-gray-400 hover:bg-gray-800/80 hover:text-white'
                  }`}
                  title={tab.label}
                >
                  <i className={`fas ${tab.icon} w-5`}></i>
                </Link>
              ) : (
                <span
                  key={tab.key}
                  className="flex justify-center py-2 rounded-lg mx-2 text-gray-500 cursor-not-allowed opacity-70"
                  title={tab.label}
                >
                  <i className={`fas ${tab.icon} w-5`}></i>
                </span>
              )
            })}
          </div>
        )}
      </nav>
    </aside>
  )
}
