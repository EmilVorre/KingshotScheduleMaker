import { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { Link } from 'react-router-dom'

interface ProfileMenuProps {
  open: boolean
  onClose: () => void
  accountName: string
  onLogout: () => void
  trigger: React.ReactNode
}

export default function ProfileMenu({ open, onClose, accountName, onLogout, trigger }: ProfileMenuProps) {
  const triggerRef = useRef<HTMLDivElement>(null)
  const [position, setPosition] = useState({ top: 0, left: 0 })

  useEffect(() => {
    if (open && triggerRef.current) {
      const rect = triggerRef.current.getBoundingClientRect()
      setPosition({
        top: rect.bottom + 4,
        left: rect.left + 12,
      })
    }
  }, [open])

  if (!open) {
    return <div ref={triggerRef}>{trigger}</div>
  }

  return (
    <>
      <div ref={triggerRef}>{trigger}</div>
      {createPortal(
        <>
          <div
            className="fixed inset-0 z-40"
            onClick={onClose}
            aria-hidden="true"
          />
          <div
            className="fixed z-50 min-w-[160px] py-1 bg-gray-800 border border-gray-600 rounded-lg shadow-xl"
            style={{ top: position.top, left: position.left }}
          >
            <Link
              to={`/dashboard/${accountName}?tab=profile`}
              onClick={onClose}
              className="flex items-center gap-2 px-4 py-2 text-gray-300 hover:bg-gray-700 hover:text-white transition-colors"
            >
              <i className="fas fa-user w-4"></i>
              Profile
            </Link>
            <button
              onClick={() => {
                onClose()
                onLogout()
              }}
              className="w-full flex items-center gap-2 px-4 py-2 text-gray-300 hover:bg-gray-700 hover:text-red-400 transition-colors text-left"
            >
              <i className="fas fa-sign-out-alt w-4"></i>
              Logout
            </button>
          </div>
        </>,
        document.body
      )}
    </>
  )
}
