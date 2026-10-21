(function() {
    'use strict';
    
    function addCopyButtons() {
        const codeBlocks = document.querySelectorAll('pre code');
        
        codeBlocks.forEach(function(codeBlock) {
            const pre = codeBlock.parentNode;
            
            if (pre.querySelector('.copy-button')) {
                return;
            }
            
            const button = document.createElement('button');
            button.className = 'copy-button';
            button.type = 'button';
            button.setAttribute('aria-label', 'Copy code to clipboard');
            button.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>';
            
            button.addEventListener('click', function() {
                const code = codeBlock.textContent;
                
                navigator.clipboard.writeText(code).then(function() {
                    button.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>';
                    button.classList.add('copied');
                    
                    setTimeout(function() {
                        button.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>';
                        button.classList.remove('copied');
                    }, 2000);
                }).catch(function(err) {
                    console.error('Failed to copy:', err);
                    button.textContent = 'Error';
                    setTimeout(function() {
                        button.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>';
                    }, 2000);
                });
            });
            
            pre.style.position = 'relative';
            pre.insertBefore(button, codeBlock);
        });
    }
    
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', addCopyButtons);
    } else {
        addCopyButtons();
    }
    
    const observer = new MutationObserver(function(mutations) {
        mutations.forEach(function(mutation) {
            if (mutation.addedNodes.length) {
                addCopyButtons();
            }
        });
    });
    
    observer.observe(document.body, {
        childList: true,
        subtree: true
    });
})();
