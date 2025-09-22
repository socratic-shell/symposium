// Hide menu bar on introduction page for cleaner landing page
document.addEventListener('DOMContentLoaded', function() {
    if (window.location.pathname.endsWith('introduction.html') || 
        window.location.pathname === '/' || 
        window.location.pathname.endsWith('/')) {
        const menuBar = document.getElementById('menu-bar');
        if (menuBar) {
            menuBar.style.display = 'none';
            // Add data attribute to body for CSS targeting
            document.body.setAttribute('data-no-menu-bar', 'true');
        }
    }
});
